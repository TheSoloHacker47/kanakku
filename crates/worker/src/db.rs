//! Read queries. Each page costs exactly one D1 round trip: related queries go out as one batch.

use kanakku_core::flags::FlagKind;
use serde::Deserialize;
use worker::d1::{D1Database, D1PreparedStatement};
use worker::wasm_bindgen::JsValue;
use worker::{query, Result};

pub const PAGE_SIZE: u32 = 30;

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub q: String,
    pub department: String,
    pub constituency: String,
    pub status: String,
    /// `any`, or a flag type.
    pub flag: String,
    /// 1-based.
    pub page: u32,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.q.is_empty() && self.department.is_empty() && self.constituency.is_empty() && self.status.is_empty() && self.flag.is_empty()
    }
}

#[derive(Debug, Deserialize)]
pub struct ListRow {
    pub code: String,
    pub title_en: String,
    pub executing_agency: Option<String>,
    pub estimated_amount: Option<i64>,
    pub expenditure: Option<i64>,
    pub works_amount: Option<i64>,
    pub estimate_shared_by: u32,
    pub official_status: Option<String>,
    pub constituencies: Option<String>,
    pub flag_types: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Facet {
    pub v: String,
    pub n: u32,
}

#[derive(Debug, Default, Deserialize)]
pub struct Totals {
    pub total: u32,
    pub flagged: u32,
    pub last_checked: Option<String>,
}

pub struct Listing {
    pub rows: Vec<ListRow>,
    pub totals: Totals,
    pub departments: Vec<Facet>,
    pub constituencies: Vec<Facet>,
    pub statuses: Vec<Facet>,
}

pub async fn list(db: &D1Database, filter: &Filter) -> Result<Listing> {
    let (clause, binds) = where_clause(filter);

    let mut page_binds = binds.clone();
    page_binds.push(JsValue::from_f64(PAGE_SIZE as f64));
    page_binds.push(JsValue::from_f64((filter.page.saturating_sub(1) * PAGE_SIZE) as f64));
    let (limit, offset) = (binds.len() + 1, binds.len() + 2);

    let rows = db
        .prepare(format!(
            "SELECT p.code, p.title_en, p.executing_agency, p.estimated_amount, p.expenditure, p.works_amount, p.estimate_shared_by, p.official_status,
                    (SELECT group_concat(name, ', ') FROM project_constituencies c WHERE c.project_id = p.id) AS constituencies,
                    (SELECT group_concat(DISTINCT type) FROM flags f WHERE f.project_id = p.id AND f.status = 'open') AS flag_types
             FROM projects p {clause}
             ORDER BY p.flag_count DESC, p.headline_amount DESC, p.code
             LIMIT ?{limit} OFFSET ?{offset}"
        ))
        .bind(&page_binds)?;
    let totals = db
        .prepare(format!(
            "SELECT COUNT(*) AS total, COALESCE(SUM(p.flag_count > 0), 0) AS flagged,
                    (SELECT last_scraped_at FROM sources WHERE id = 1) AS last_checked
             FROM projects p {clause}"
        ))
        .bind(&binds)?;

    let results = db
        .batch(vec![
            rows,
            totals,
            facet(db, "SELECT department AS v, COUNT(*) AS n FROM projects WHERE department IS NOT NULL GROUP BY 1 ORDER BY 1"),
            facet(db, "SELECT name AS v, COUNT(*) AS n FROM project_constituencies GROUP BY 1 ORDER BY 1"),
            facet(db, "SELECT official_status AS v, COUNT(*) AS n FROM projects WHERE official_status IS NOT NULL GROUP BY 1 ORDER BY 2 DESC"),
        ])
        .await?;

    Ok(Listing {
        rows: results[0].results()?,
        totals: results[1].results::<Totals>()?.into_iter().next().unwrap_or_default(),
        departments: results[2].results()?,
        constituencies: results[3].results()?,
        statuses: results[4].results()?,
    })
}

fn facet(db: &D1Database, sql: &str) -> D1PreparedStatement {
    db.prepare(sql)
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
    if !filter.status.is_empty() {
        parts.push(format!("p.official_status = ?{}", bind(&filter.status)));
    }
    if filter.flag == "any" {
        parts.push("p.flag_count > 0".into());
    } else if let Some(kind) = FlagKind::parse(&filter.flag) {
        parts.push(format!("p.id IN (SELECT project_id FROM flags WHERE status = 'open' AND type = ?{})", bind(kind.as_str())));
    }

    let clause = if parts.is_empty() { String::new() } else { format!("WHERE {}", parts.join(" AND ")) };
    (clause, binds)
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

pub struct ProjectPage {
    pub row: ProjectRow,
    pub flags: Vec<FlagRow>,
    pub observations: Vec<ObservationRow>,
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
        ])
        .await?;

    let Some(row) = results[0].results::<ProjectRow>()?.into_iter().next() else { return Ok(None) };
    Ok(Some(ProjectPage { row, flags: results[1].results()?, observations: results[2].results()? }))
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
