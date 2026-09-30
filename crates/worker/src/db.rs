//! Read queries. Each page costs exactly one D1 round trip: related queries go out as one batch.

use kanakku_core::flags::FlagKind;
use kanakku_core::stage::Stage;
use serde::Deserialize;
use worker::d1::D1Database;
use worker::wasm_bindgen::JsValue;
use worker::{query, Result};

pub const PAGE_SIZE: u32 = 24;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    /// Flagged projects first, then the largest.
    #[default]
    Flags,
    Amount,
    Spent,
    Name,
    Changed,
}

impl Sort {
    pub const ALL: [Sort; 5] = [Sort::Flags, Sort::Amount, Sort::Spent, Sort::Name, Sort::Changed];

    pub fn as_str(self) -> &'static str {
        match self {
            Sort::Flags => "flags",
            Sort::Amount => "amount",
            Sort::Spent => "spent",
            Sort::Name => "name",
            Sort::Changed => "changed",
        }
    }

    pub fn parse(s: &str) -> Sort {
        Sort::ALL.into_iter().find(|sort| sort.as_str() == s).unwrap_or_default()
    }

    fn order_by(self) -> &'static str {
        match self {
            Sort::Flags => "p.flag_count DESC, p.headline_amount DESC, p.code",
            Sort::Amount => "p.headline_amount DESC, p.code",
            Sort::Spent => "COALESCE(p.expenditure, 0) DESC, p.code",
            Sort::Name => "p.title_en COLLATE NOCASE, p.code",
            Sort::Changed => "p.changed_on DESC, p.flag_count DESC, p.code",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub q: String,
    pub department: String,
    pub constituency: String,
    /// A stage name, see `Stage::as_str`.
    pub stage: String,
    /// `any`, or a flag type.
    pub flag: String,
    pub sort: Sort,
    /// 1-based.
    pub page: u32,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.q.is_empty() && self.department.is_empty() && self.constituency.is_empty() && self.stage.is_empty() && self.flag.is_empty()
    }
}

/// One project as a list row.
#[derive(Debug, Deserialize)]
pub struct ListRow {
    pub code: String,
    pub title_en: String,
    pub department: Option<String>,
    pub estimated_amount: Option<i64>,
    pub expenditure: Option<i64>,
    pub works_amount: Option<i64>,
    pub estimate_shared_by: u32,
    pub stage: Option<String>,
    pub constituencies: Option<String>,
    pub flag_types: Option<String>,
}

const LIST_COLUMNS: &str = "p.code, p.title_en, p.department, p.estimated_amount, p.expenditure, p.works_amount,
    p.estimate_shared_by, p.stage,
    (SELECT group_concat(name, '|') FROM project_constituencies c WHERE c.project_id = p.id) AS constituencies,
    (SELECT group_concat(DISTINCT type) FROM flags f WHERE f.project_id = p.id AND f.status = 'open') AS flag_types";

#[derive(Debug, Deserialize)]
pub struct Facet {
    pub v: String,
    pub n: u32,
    #[serde(default)]
    pub spent: Option<i64>,
    #[serde(default)]
    pub flagged: Option<u32>,
    /// For constituencies: the MLA as KIIFB lists them.
    #[serde(default)]
    pub mla: Option<String>,
    #[serde(default)]
    pub mla_ml: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Totals {
    pub total: u32,
    pub flagged: u32,
    pub spent: Option<i64>,
    pub last_checked: Option<String>,
}

pub struct Listing {
    pub rows: Vec<ListRow>,
    pub totals: Totals,
    pub departments: Vec<Facet>,
    pub constituencies: Vec<Facet>,
    pub stages: Vec<Facet>,
}

const DEPARTMENT_FACET: &str = "SELECT department AS v, COUNT(*) AS n, SUM(expenditure) AS spent, SUM(flag_count > 0) AS flagged
    FROM projects WHERE department IS NOT NULL GROUP BY 1 ORDER BY 2 DESC, 1";
const CONSTITUENCY_FACET: &str = "SELECT c.name AS v, COUNT(*) AS n, SUM(p.expenditure) AS spent, SUM(p.flag_count > 0) AS flagged,
        MAX(c.mla_name) AS mla, MAX(c.mla_name_ml) AS mla_ml
    FROM project_constituencies c JOIN projects p ON p.id = c.project_id GROUP BY 1 ORDER BY 1";
const STAGE_FACET: &str = "SELECT stage AS v, COUNT(*) AS n FROM projects WHERE stage IS NOT NULL GROUP BY 1";
const TOTALS: &str = "SELECT COUNT(*) AS total, COALESCE(SUM(p.flag_count > 0), 0) AS flagged, SUM(p.expenditure) AS spent,
    (SELECT last_scraped_at FROM sources WHERE id = 1) AS last_checked FROM projects p";

pub async fn list(db: &D1Database, filter: &Filter) -> Result<Listing> {
    let (clause, binds) = where_clause(filter);

    let mut page_binds = binds.clone();
    page_binds.push(JsValue::from_f64(PAGE_SIZE as f64));
    page_binds.push(JsValue::from_f64((filter.page.saturating_sub(1) * PAGE_SIZE) as f64));
    let (limit, offset) = (binds.len() + 1, binds.len() + 2);

    let rows = db
        .prepare(format!(
            "SELECT {LIST_COLUMNS} FROM projects p {clause} ORDER BY {} LIMIT ?{limit} OFFSET ?{offset}",
            filter.sort.order_by()
        ))
        .bind(&page_binds)?;
    let totals = db.prepare(format!("{TOTALS} {clause}")).bind(&binds)?;

    let results = db
        .batch(vec![rows, totals, db.prepare(DEPARTMENT_FACET), db.prepare(CONSTITUENCY_FACET), db.prepare(STAGE_FACET)])
        .await?;

    Ok(Listing {
        rows: results[0].results()?,
        totals: results[1].results::<Totals>()?.into_iter().next().unwrap_or_default(),
        departments: results[2].results()?,
        constituencies: results[3].results()?,
        stages: results[4].results()?,
    })
}

/// Builds the WHERE clause for a filter. Every user value is bound, never interpolated.
fn where_clause(filter: &Filter) -> (String, Vec<JsValue>) {
    let mut parts: Vec<String> = Vec::new();
    let mut binds: Vec<JsValue> = Vec::new();
    let mut bind = |value: &str| {
        binds.push(JsValue::from_str(value));
        binds.len()
    };

    let terms: Vec<&str> = filter.q.split_whitespace().collect();
    if !terms.is_empty() {
        // The trigram index needs three characters per term; shorter searches fall back to LIKE.
        if terms.iter().all(|t| t.chars().count() >= 3) {
            let phrase = terms.iter().map(|t| format!("\"{}\"", t.replace('"', "\"\""))).collect::<Vec<_>>().join(" ");
            parts.push(format!("p.id IN (SELECT rowid FROM projects_fts WHERE projects_fts MATCH ?{})", bind(&phrase)));
        } else {
            let like = format!("%{}%", filter.q.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            let n = bind(&like);
            parts.push(format!("(p.title_en LIKE ?{n} ESCAPE '\\' OR p.code LIKE ?{n} ESCAPE '\\')"));
        }
    }
    if !filter.department.is_empty() {
        parts.push(format!("p.department = ?{}", bind(&filter.department)));
    }
    if !filter.constituency.is_empty() {
        parts.push(format!("p.id IN (SELECT project_id FROM project_constituencies WHERE name = ?{})", bind(&filter.constituency)));
    }
    if let Some(stage) = Stage::parse(&filter.stage) {
        parts.push(format!("p.stage = ?{}", bind(stage.as_str())));
    }
    if filter.flag == "any" {
        parts.push("p.flag_count > 0".into());
    } else if let Some(kind) = FlagKind::parse(&filter.flag) {
        parts.push(format!("p.id IN (SELECT project_id FROM flags WHERE status = 'open' AND type = ?{})", bind(kind.as_str())));
    }

    let clause = if parts.is_empty() { String::new() } else { format!("WHERE {}", parts.join(" AND ")) };
    (clause, binds)
}

/// A project location for the small district map.
#[derive(Debug, Deserialize)]
pub struct Point {
    pub lat: f64,
    pub lng: f64,
    pub flagged: u32,
}

#[derive(Debug, Deserialize)]
pub struct RecentChange {
    pub code: String,
    pub title_en: String,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub observed_on: String,
}

pub struct Home {
    pub totals: Totals,
    pub flagged: Vec<ListRow>,
    pub largest: Vec<ListRow>,
    pub departments: Vec<Facet>,
    pub constituencies: Vec<Facet>,
    pub stages: Vec<Facet>,
    pub points: Vec<Point>,
    pub changes: Vec<RecentChange>,
}

pub async fn home(db: &D1Database) -> Result<Home> {
    let results = db
        .batch(vec![
            db.prepare(TOTALS),
            db.prepare(format!(
                "SELECT {LIST_COLUMNS} FROM projects p WHERE p.flag_count > 0 ORDER BY p.flag_count DESC, p.headline_amount DESC LIMIT 6"
            )),
            db.prepare(format!("SELECT {LIST_COLUMNS} FROM projects p ORDER BY COALESCE(p.expenditure, 0) DESC LIMIT 5")),
            db.prepare(DEPARTMENT_FACET),
            db.prepare(CONSTITUENCY_FACET),
            db.prepare(STAGE_FACET),
            db.prepare(
                "SELECT s.lat, s.lng, p.flag_count > 0 AS flagged FROM sites s JOIN projects p ON p.id = s.project_id
                 UNION ALL
                 SELECT w.lat, w.lng, p.flag_count > 0 FROM works w JOIN projects p ON p.id = w.project_id WHERE w.lat IS NOT NULL",
            ),
            db.prepare(
                "SELECT p.code, p.title_en, o.field, o.old_value, o.new_value, o.observed_on
                 FROM observations o JOIN projects p ON p.id = o.project_id ORDER BY o.id DESC LIMIT 6",
            ),
        ])
        .await?;
    Ok(Home {
        totals: results[0].results::<Totals>()?.into_iter().next().unwrap_or_default(),
        flagged: results[1].results()?,
        largest: results[2].results()?,
        departments: results[3].results()?,
        constituencies: results[4].results()?,
        stages: results[5].results()?,
        points: results[6].results()?,
        changes: results[7].results()?,
    })
}

#[derive(Debug, Deserialize)]
pub struct ProjectRow {
    pub record_json: String,
    pub first_seen_on: String,
    pub missing_since: Option<String>,
    pub snapshot_id: i64,
    pub sha256: String,
    pub fetched_at: String,
    pub last_checked: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FlagRow {
    #[serde(rename = "type")]
    pub kind: String,
    pub work_ref: Option<String>,
    pub rule_version: u32,
    pub value_json: String,
    pub status: String,
    pub created_on: String,
    pub cleared_on: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ObservationRow {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub observed_on: String,
}

/// Another package under the same sub-project.
#[derive(Debug, Deserialize)]
pub struct Sibling {
    pub code: String,
    pub title_en: String,
    pub expenditure: Option<i64>,
    pub flag_count: u32,
}

pub struct ProjectPage {
    pub row: ProjectRow,
    pub flags: Vec<FlagRow>,
    pub observations: Vec<ObservationRow>,
    pub siblings: Vec<Sibling>,
}

pub async fn project(db: &D1Database, code: &str) -> Result<Option<ProjectPage>> {
    const ID: &str = "(SELECT id FROM projects WHERE code = ?1)";
    let results = db
        .batch(vec![
            query!(
                db,
                "SELECT p.record_json, p.first_seen_on, p.missing_since, s.id AS snapshot_id, s.sha256, s.fetched_at,
                        (SELECT last_scraped_at FROM sources WHERE id = 1) AS last_checked
                 FROM projects p JOIN snapshots s ON s.id = p.snapshot_id WHERE p.code = ?1",
                code,
            )?,
            query!(
                db,
                &format!(
                    "SELECT type, work_ref, rule_version, value_json, status, created_on, cleared_on
                     FROM flags WHERE project_id = {ID} ORDER BY status = 'open' DESC, id DESC LIMIT 40"
                ),
                code,
            )?,
            query!(
                db,
                &format!("SELECT field, old_value, new_value, observed_on FROM observations WHERE project_id = {ID} ORDER BY id DESC LIMIT 40"),
                code,
            )?,
            query!(
                db,
                "SELECT code, title_en, expenditure, flag_count FROM projects
                 WHERE code != ?1 AND estimated_amount IS NOT NULL
                   AND sub_project_code = (SELECT sub_project_code FROM projects WHERE code = ?1)
                   AND estimated_amount = (SELECT estimated_amount FROM projects WHERE code = ?1)
                 ORDER BY COALESCE(expenditure, 0) DESC, code LIMIT 8",
                code,
            )?,
        ])
        .await?;

    let Some(row) = results[0].results::<ProjectRow>()?.into_iter().next() else { return Ok(None) };
    Ok(Some(ProjectPage { row, flags: results[1].results()?, observations: results[2].results()?, siblings: results[3].results()? }))
}

pub async fn last_checked(db: &D1Database) -> Result<Option<String>> {
    db.prepare("SELECT last_scraped_at FROM sources WHERE id = 1").first::<String>(Some("last_scraped_at")).await
}

#[derive(Debug, Deserialize)]
pub struct SnapshotRow {
    pub r2_key: String,
    pub sha256: String,
}

pub async fn snapshot(db: &D1Database, id: i64) -> Result<Option<SnapshotRow>> {
    query!(db, "SELECT r2_key, sha256 FROM snapshots WHERE id = ?1", id)?.first(None).await
}

/// Every project with its open flags, for the open-data endpoints.
#[derive(Debug, Deserialize)]
pub struct ExportRow {
    pub record_json: String,
    pub first_seen_on: String,
    pub changed_on: String,
    pub missing_since: Option<String>,
    pub flag_types: Option<String>,
}

pub async fn export(db: &D1Database) -> Result<Vec<ExportRow>> {
    db.prepare(
        "SELECT p.record_json, p.first_seen_on, p.changed_on, p.missing_since,
                (SELECT group_concat(DISTINCT type) FROM flags f WHERE f.project_id = p.id AND f.status = 'open') AS flag_types
         FROM projects p ORDER BY p.code",
    )
    .all()
    .await?
    .results()
}
