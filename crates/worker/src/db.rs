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
    /// A district name, `-` for projects KIIFB files under no district, or empty for the whole state.
    pub district: String,
    /// An implementing agency's key.
    pub agency: String,
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
        self.q.is_empty()
            && self.district.is_empty()
            && self.agency.is_empty()
            && self.department.is_empty()
            && self.constituency.is_empty()
            && self.stage.is_empty()
            && self.flag.is_empty()
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
    pub districts: Vec<Facet>,
    /// The name of the agency the list is narrowed to, as a source spells it.
    pub agency_name: Option<String>,
}

/// Narrows `projects p` to a district: `?1` is a district name, `-` for none stated, or empty for all.
const IN_DISTRICT: &str = "(?1 = '' OR (?1 = '-' AND p.district = '') OR p.id IN (SELECT project_id FROM project_districts WHERE district = ?1))";

fn department_facet() -> String {
    format!(
        "SELECT department AS v, COUNT(*) AS n, SUM(expenditure) AS spent, SUM(flag_count > 0) AS flagged
         FROM projects p WHERE department IS NOT NULL AND {IN_DISTRICT} GROUP BY 1 ORDER BY 2 DESC, 1"
    )
}

/// Constituencies of one district. For the whole state the list would be 140 long, so it is left empty.
const CONSTITUENCY_FACET: &str = "SELECT c.name AS v, COUNT(*) AS n, SUM(p.expenditure) AS spent, SUM(p.flag_count > 0) AS flagged,
        MAX(c.mla_name) AS mla, MAX(c.mla_name_ml) AS mla_ml
    FROM project_constituencies c JOIN projects p ON p.id = c.project_id WHERE c.district = ?1 GROUP BY 1 ORDER BY 1";

fn stage_facet() -> String {
    format!("SELECT stage AS v, COUNT(*) AS n FROM projects p WHERE stage IS NOT NULL AND {IN_DISTRICT} GROUP BY 1")
}

/// Every district with its projects, and the projects filed under none as the district `-`.
const DISTRICT_FACET: &str = "SELECT d.district AS v, COUNT(*) AS n, SUM(p.expenditure) AS spent, SUM(p.flag_count > 0) AS flagged
      FROM project_districts d JOIN projects p ON p.id = d.project_id GROUP BY 1
    UNION ALL
    SELECT '-', COUNT(*), SUM(expenditure), SUM(flag_count > 0) FROM projects WHERE district = '' HAVING COUNT(*) > 0";

fn totals(clause: &str) -> String {
    format!(
        "SELECT COUNT(*) AS total, COALESCE(SUM(p.flag_count > 0), 0) AS flagged, SUM(p.expenditure) AS spent,
                (SELECT last_scraped_at FROM sources WHERE id = 1) AS last_checked FROM projects p {clause}"
    )
}

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
    let district = [JsValue::from_str(&filter.district)];

    let results = db
        .batch(vec![
            rows,
            db.prepare(totals(&clause)).bind(&binds)?,
            db.prepare(department_facet()).bind(&district)?,
            db.prepare(CONSTITUENCY_FACET).bind(&district)?,
            db.prepare(stage_facet()).bind(&district)?,
            db.prepare(DISTRICT_FACET),
            query!(db, "SELECT executing_agency AS name FROM projects WHERE agency_key = ?1 AND ?1 != '' LIMIT 1", filter.agency)?,
        ])
        .await?;

    #[derive(Deserialize)]
    struct Name {
        name: Option<String>,
    }
    Ok(Listing {
        rows: results[0].results()?,
        totals: results[1].results::<Totals>()?.into_iter().next().unwrap_or_default(),
        departments: results[2].results()?,
        constituencies: results[3].results()?,
        stages: results[4].results()?,
        districts: results[5].results()?,
        agency_name: results[6].results::<Name>()?.into_iter().next().and_then(|row| row.name),
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
    if filter.district == "-" {
        parts.push("p.district = ''".into());
    } else if !filter.district.is_empty() {
        parts.push(format!("p.id IN (SELECT project_id FROM project_districts WHERE district = ?{})", bind(&filter.district)));
    }
    if !filter.agency.is_empty() {
        parts.push(format!("p.agency_key = ?{}", bind(&filter.agency)));
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
    pub funding: FundingTotals,
    /// Works currently on PWD's defect-liability list.
    pub liability: u32,
    /// Every district, for the state's front page.
    pub districts: Vec<Facet>,
}

#[derive(Deserialize)]
struct Count {
    n: u32,
}

/// The front page for the whole state (`district` empty) or for one district.
pub async fn home(db: &D1Database, district: &str) -> Result<Home> {
    let results = db
        .batch(vec![
            query!(db, &totals(&format!("WHERE {IN_DISTRICT}")), district)?,
            query!(
                db,
                &format!(
                    "SELECT {LIST_COLUMNS} FROM projects p WHERE p.flag_count > 0 AND {IN_DISTRICT}
                     ORDER BY p.flag_count DESC, p.headline_amount DESC LIMIT 6"
                ),
                district,
            )?,
            query!(
                db,
                &format!("SELECT {LIST_COLUMNS} FROM projects p WHERE {IN_DISTRICT} ORDER BY COALESCE(p.expenditure, 0) DESC LIMIT 5"),
                district,
            )?,
            query!(db, &department_facet(), district)?,
            query!(db, CONSTITUENCY_FACET, district)?,
            query!(db, &stage_facet(), district)?,
            query!(
                db,
                &format!(
                    "SELECT s.lat, s.lng, p.flag_count > 0 AS flagged FROM sites s JOIN projects p ON p.id = s.project_id WHERE {IN_DISTRICT}
                     UNION ALL
                     SELECT w.lat, w.lng, p.flag_count > 0 FROM works w JOIN projects p ON p.id = w.project_id
                      WHERE w.lat IS NOT NULL AND {IN_DISTRICT}"
                ),
                district,
            )?,
            query!(
                db,
                &format!(
                    "SELECT p.code, p.title_en, o.field, o.old_value, o.new_value, o.observed_on
                     FROM observations o JOIN projects p ON p.id = o.project_id WHERE {IN_DISTRICT} ORDER BY o.id DESC LIMIT 6"
                ),
                district,
            )?,
            query!(db, &funding_totals(), district)?,
            // Today in India, since liability dates are Indian calendar days.
            query!(
                db,
                "SELECT COUNT(*) AS n FROM liability_works
                 WHERE missing_since IS NULL AND ends_on >= date('now', '+330 minutes') AND (?1 = '' OR district = ?1)",
                district,
            )?,
            db.prepare(DISTRICT_FACET),
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
        funding: results[8].results::<FundingTotals>()?.into_iter().next().unwrap_or_default(),
        liability: results[9].results::<Count>()?.into_iter().next().map(|row| row.n).unwrap_or(0),
        districts: results[10].results()?,
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

/// The KIIFB status-page project a map package was joined to.
#[derive(Debug, Deserialize)]
pub struct FundingLink {
    pub record_json: String,
    pub detail_checked_at: Option<String>,
    /// Positions of the works whose title matches this package, comma-separated.
    pub own_works: Option<String>,
}

pub struct ProjectPage {
    pub row: ProjectRow,
    pub flags: Vec<FlagRow>,
    pub observations: Vec<ObservationRow>,
    pub siblings: Vec<Sibling>,
    pub funding: Option<FundingLink>,
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
            query!(
                db,
                &format!(
                    "SELECT f.record_json, f.detail_checked_at,
                            (SELECT group_concat(w.seq) FROM funding_works w
                              WHERE w.funding_project_id = f.id AND w.project_code = ?1) AS own_works
                     FROM funding_projects f JOIN projects p ON {FUNDING_JOIN}
                     WHERE p.code = ?1 AND f.missing_since IS NULL LIMIT 1"
                ),
                code,
            )?,
        ])
        .await?;

    let Some(row) = results[0].results::<ProjectRow>()?.into_iter().next() else { return Ok(None) };
    Ok(Some(ProjectPage {
        row,
        flags: results[1].results()?,
        observations: results[2].results()?,
        siblings: results[3].results()?,
        funding: results[4].results::<FundingLink>()?.into_iter().next(),
    }))
}

pub async fn last_checked(db: &D1Database) -> Result<Option<String>> {
    db.prepare("SELECT last_scraped_at FROM sources WHERE id = 1").first::<String>(Some("last_scraped_at")).await
}

#[derive(Debug, Deserialize)]
pub struct SnapshotRow {
    pub r2_key: String,
}

pub async fn snapshot(db: &D1Database, id: i64) -> Result<Option<SnapshotRow>> {
    query!(db, "SELECT r2_key FROM snapshots WHERE id = ?1", id)?.first(None).await
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

/// A status-page project belongs to the map packages filed under the same sub-project with the same estimate.
const FUNDING_JOIN: &str = "f.group_key = COALESCE(p.sub_project_code, p.code) AND f.approved_amount = p.estimated_amount";

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FundingSort {
    #[default]
    Approved,
    Released,
    /// Approved but not yet released.
    Balance,
    Name,
}

impl FundingSort {
    pub const ALL: [FundingSort; 4] = [FundingSort::Approved, FundingSort::Released, FundingSort::Balance, FundingSort::Name];

    pub fn as_str(self) -> &'static str {
        match self {
            FundingSort::Approved => "approved",
            FundingSort::Released => "released",
            FundingSort::Balance => "balance",
            FundingSort::Name => "name",
        }
    }

    pub fn parse(s: &str) -> FundingSort {
        FundingSort::ALL.into_iter().find(|sort| sort.as_str() == s).unwrap_or_default()
    }

    fn order_by(self) -> &'static str {
        match self {
            FundingSort::Approved => "COALESCE(f.approved_amount, 0) DESC, f.name",
            FundingSort::Released => "COALESCE(f.released_amount, 0) DESC, f.name",
            FundingSort::Balance => "COALESCE(f.approved_amount, 0) - COALESCE(f.released_amount, 0) DESC, f.name",
            FundingSort::Name => "f.name COLLATE NOCASE",
        }
    }
}

/// One project from KIIFB's status page, as a list row.
#[derive(Debug, Deserialize)]
pub struct FundingRow {
    #[serde(rename = "ref")]
    pub reference: String,
    pub name: String,
    pub department: Option<String>,
    pub spv: Option<String>,
    pub approved_amount: Option<i64>,
    pub released_amount: Option<i64>,
    pub work_count: u32,
    pub flag_count: u32,
    /// Whether the project's work table has been read yet.
    pub read: u32,
    /// Map packages this project was joined to.
    pub packages: u32,
}

#[derive(Debug, Default, Deserialize)]
pub struct FundingTotals {
    pub total: u32,
    pub approved: Option<i64>,
    pub released: Option<i64>,
    pub evaluating: u32,
    pub works: u32,
    /// Projects whose work table has not been read yet, so whose paid figure is still KIIFB's listed one.
    pub unread: u32,
    pub flagged: u32,
    pub last_checked: Option<String>,
}

/// Narrows `funding_projects f` to a district; `?1` is a district name or empty for the whole state.
const FUNDED_IN_DISTRICT: &str = "(?1 = '' OR f.id IN (SELECT funding_project_id FROM funding_districts WHERE district = ?1))";

fn funding_totals() -> String {
    format!(
        "SELECT COUNT(*) AS total, SUM(approved_amount) AS approved, SUM(released_amount) AS released,
                COALESCE(SUM(approved_amount IS NULL), 0) AS evaluating, COALESCE(SUM(work_count), 0) AS works,
                COALESCE(SUM(detail_checked_at IS NULL), 0) AS unread, COALESCE(SUM(flag_count > 0), 0) AS flagged,
                (SELECT last_scraped_at FROM sources WHERE id = 2) AS last_checked
         FROM funding_projects f WHERE missing_since IS NULL AND {FUNDED_IN_DISTRICT}"
    )
}

pub const FUNDING_PAGE_SIZE: u32 = 40;

pub struct FundingListing {
    pub rows: Vec<FundingRow>,
    pub totals: FundingTotals,
    /// How many projects match the search and filters, for paging.
    pub matching: u32,
}

pub async fn funding_list(db: &D1Database, sort: FundingSort, district: &str, q: &str, flagged: bool, page: u32) -> Result<FundingListing> {
    let like = format!("%{}%", q.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let narrowed = format!(
        "f.missing_since IS NULL AND {FUNDED_IN_DISTRICT} AND (?2 = '%%' OR f.name LIKE ?2 ESCAPE '\\' OR f.spv LIKE ?2 ESCAPE '\\')
         AND (?3 = 0 OR f.flag_count > 0)"
    );
    let results = db
        .batch(vec![
            query!(
                db,
                &format!(
                    "SELECT f.ref, f.name, f.department, f.spv, f.approved_amount, f.released_amount, f.work_count, f.flag_count,
                            f.detail_checked_at IS NOT NULL AS read,
                            (SELECT COUNT(*) FROM projects p WHERE {FUNDING_JOIN}) AS packages
                     FROM funding_projects f WHERE {narrowed} ORDER BY {} LIMIT ?4 OFFSET ?5",
                    sort.order_by()
                ),
                district,
                like,
                flagged,
                FUNDING_PAGE_SIZE,
                page.saturating_sub(1) * FUNDING_PAGE_SIZE,
            )?,
            query!(db, &funding_totals(), district)?,
            query!(db, &format!("SELECT COUNT(*) AS n FROM funding_projects f WHERE {narrowed}"), district, like, flagged)?,
        ])
        .await?;
    Ok(FundingListing {
        rows: results[0].results()?,
        totals: results[1].results::<FundingTotals>()?.into_iter().next().unwrap_or_default(),
        matching: results[2].results::<Count>()?.into_iter().next().map(|row| row.n).unwrap_or(0),
    })
}

#[derive(Debug, Deserialize)]
pub struct FundingDetailRow {
    pub record_json: String,
    pub first_seen_on: String,
    pub missing_since: Option<String>,
    pub match_basis: Option<String>,
    pub snapshot_id: i64,
    pub fetched_at: String,
    pub detail_snapshot_id: Option<i64>,
    pub detail_checked_at: Option<String>,
    pub last_checked: Option<String>,
}

/// A work whose title matches a map package.
#[derive(Debug, Deserialize)]
pub struct WorkLink {
    pub seq: u32,
    pub project_code: String,
}

pub struct FundingPage {
    pub row: FundingDetailRow,
    pub links: Vec<WorkLink>,
    pub packages: Vec<Sibling>,
    pub observations: Vec<ObservationRow>,
    pub flags: Vec<FlagRow>,
}

pub async fn funding_project(db: &D1Database, reference: &str) -> Result<Option<FundingPage>> {
    const ID: &str = "(SELECT id FROM funding_projects WHERE ref = ?1)";
    let results = db
        .batch(vec![
            query!(
                db,
                "SELECT f.record_json, f.first_seen_on, f.missing_since, f.match_basis, f.snapshot_id, s.fetched_at,
                        f.detail_snapshot_id, f.detail_checked_at,
                        (SELECT last_scraped_at FROM sources WHERE id = 2) AS last_checked
                 FROM funding_projects f JOIN snapshots s ON s.id = f.snapshot_id WHERE f.ref = ?1",
                reference,
            )?,
            query!(
                db,
                &format!("SELECT seq, project_code FROM funding_works WHERE funding_project_id = {ID} AND project_code IS NOT NULL"),
                reference,
            )?,
            query!(
                db,
                &format!(
                    "SELECT p.code, p.title_en, p.expenditure, p.flag_count FROM projects p JOIN funding_projects f ON {FUNDING_JOIN}
                     WHERE f.ref = ?1 ORDER BY COALESCE(p.expenditure, 0) DESC, p.code LIMIT 80"
                ),
                reference,
            )?,
            query!(
                db,
                &format!(
                    "SELECT field, old_value, new_value, observed_on FROM funding_observations
                     WHERE funding_project_id = {ID} ORDER BY id DESC LIMIT 40"
                ),
                reference,
            )?,
            query!(
                db,
                &format!(
                    "SELECT type, work_ref, rule_version, value_json, status, created_on, cleared_on
                     FROM funding_flags WHERE funding_project_id = {ID} ORDER BY status = 'open' DESC, id DESC LIMIT 40"
                ),
                reference,
            )?,
        ])
        .await?;
    let Some(row) = results[0].results::<FundingDetailRow>()?.into_iter().next() else { return Ok(None) };
    Ok(Some(FundingPage {
        row,
        links: results[1].results()?,
        packages: results[2].results()?,
        observations: results[3].results()?,
        flags: results[4].results()?,
    }))
}

/// Every status-page project, for the open-data endpoints.
#[derive(Debug, Deserialize)]
pub struct FundingExportRow {
    pub record_json: String,
    pub group_key: Option<String>,
    pub match_basis: Option<String>,
    pub first_seen_on: String,
    pub changed_on: String,
    pub missing_since: Option<String>,
}

pub async fn funding_export(db: &D1Database) -> Result<(Vec<FundingExportRow>, Option<String>)> {
    let results = db
        .batch(vec![
            db.prepare(
                "SELECT record_json, group_key, match_basis, first_seen_on, changed_on, missing_since
                 FROM funding_projects ORDER BY COALESCE(approved_amount, 0) DESC, name",
            ),
            db.prepare("SELECT last_scraped_at FROM sources WHERE id = 2"),
        ])
        .await?;
    #[derive(Deserialize)]
    struct Checked {
        last_scraped_at: Option<String>,
    }
    let checked = results[1].results::<Checked>()?.into_iter().next().and_then(|row| row.last_scraped_at);
    Ok((results[0].results()?, checked))
}

/// A finished work still under defect liability, as a list row.
#[derive(Debug, Deserialize)]
pub struct LiabilityRow {
    pub wing: String,
    pub name: String,
    pub contractor: Option<String>,
    pub contractor_key: Option<String>,
    pub agreed_amount: Option<i64>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub division: Option<String>,
    pub subdivision: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct LiabilityTotals {
    pub total: u32,
    pub active: u32,
    pub ending_soon: u32,
    pub contractors: u32,
    /// Works now under liability for which PWD states an agreed amount.
    pub with_amount: u32,
    pub last_checked: Option<String>,
    pub snapshot_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ContractorCount {
    pub contractor: String,
    pub contractor_key: String,
    pub n: u32,
}

pub struct Liability {
    pub rows: Vec<LiabilityRow>,
    pub totals: LiabilityTotals,
    pub wings: Vec<Facet>,
    pub districts: Vec<Facet>,
    pub contractors: Vec<ContractorCount>,
    /// How many works match the filters, for paging.
    pub matching: u32,
}

const LIABILITY_COLUMNS: &str = "wing, name, contractor, contractor_key, agreed_amount, starts_on, ends_on, division, subdivision";
pub const LIABILITY_PAGE_SIZE: u32 = 60;

#[derive(Clone, Debug, Default)]
pub struct LiabilityFilter {
    pub wing: String,
    pub contractor: String,
    /// The district of the PWD office handling the work.
    pub district: String,
    pub page: u32,
}

/// The filters narrow the rows; the totals describe the chosen district as a whole.
/// `today` and `soon` are yyyy-mm-dd: works ending between them count as ending soon.
pub async fn liability(db: &D1Database, filter: &LiabilityFilter, today: &str, soon: &str) -> Result<Liability> {
    const NARROWED: &str = "missing_since IS NULL AND (?1 = '' OR wing = ?1) AND (?2 = '' OR contractor_key = ?2) AND (?3 = '' OR district = ?3)";
    let results = db
        .batch(vec![
            query!(
                db,
                &format!(
                    "SELECT {LIABILITY_COLUMNS} FROM liability_works WHERE {NARROWED}
                     ORDER BY (ends_on IS NULL OR ends_on < ?4), ends_on, name LIMIT ?5 OFFSET ?6"
                ),
                filter.wing,
                filter.contractor,
                filter.district,
                today,
                LIABILITY_PAGE_SIZE,
                filter.page.saturating_sub(1) * LIABILITY_PAGE_SIZE,
            )?,
            query!(
                db,
                "SELECT COUNT(*) AS total, COALESCE(SUM(ends_on >= ?1), 0) AS active,
                        COALESCE(SUM(ends_on >= ?1 AND ends_on <= ?2), 0) AS ending_soon,
                        COUNT(DISTINCT contractor_key) AS contractors,
                        COALESCE(SUM(ends_on >= ?1 AND agreed_amount IS NOT NULL), 0) AS with_amount,
                        (SELECT last_scraped_at FROM sources WHERE id = 3) AS last_checked,
                        (SELECT MAX(id) FROM snapshots WHERE source_id = 3) AS snapshot_id
                 FROM liability_works WHERE missing_since IS NULL AND (?3 = '' OR district = ?3)",
                today,
                soon,
                filter.district,
            )?,
            query!(
                db,
                "SELECT wing AS v, COUNT(*) AS n FROM liability_works WHERE missing_since IS NULL AND (?1 = '' OR district = ?1)
                 GROUP BY 1 ORDER BY 2 DESC",
                filter.district,
            )?,
            db.prepare(
                "SELECT COALESCE(district, '') AS v, COUNT(*) AS n FROM liability_works WHERE missing_since IS NULL GROUP BY 1 ORDER BY 1",
            ),
            query!(
                db,
                "SELECT MAX(contractor) AS contractor, contractor_key, COUNT(*) AS n FROM liability_works
                 WHERE missing_since IS NULL AND contractor_key IS NOT NULL AND ends_on >= ?1 AND (?2 = '' OR district = ?2)
                 GROUP BY contractor_key ORDER BY n DESC, contractor LIMIT 12",
                today,
                filter.district,
            )?,
            query!(db, &format!("SELECT COUNT(*) AS n FROM liability_works WHERE {NARROWED}"), filter.wing, filter.contractor, filter.district)?,
        ])
        .await?;
    Ok(Liability {
        rows: results[0].results()?,
        totals: results[1].results::<LiabilityTotals>()?.into_iter().next().unwrap_or_default(),
        wings: results[2].results()?,
        districts: results[3].results()?,
        contractors: results[4].results()?,
        matching: results[5].results::<Count>()?.into_iter().next().map(|row| row.n).unwrap_or(0),
    })
}

/// Every work on the liability list, including those that have dropped off it, for the open-data endpoints.
#[derive(Debug, Deserialize)]
pub struct LiabilityExportRow {
    pub district: Option<String>,
    pub wing: String,
    pub name: String,
    pub contractor: Option<String>,
    pub agreed_amount: Option<i64>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub division: Option<String>,
    pub subdivision: Option<String>,
    pub first_seen_on: String,
    pub missing_since: Option<String>,
}

pub async fn liability_export(db: &D1Database) -> Result<Vec<LiabilityExportRow>> {
    db.prepare(
        "SELECT district, wing, name, contractor, agreed_amount, starts_on, ends_on, division, subdivision, first_seen_on, missing_since
         FROM liability_works ORDER BY ends_on, name",
    )
    .all()
    .await?
    .results()
}

/// One source and how its reads have gone.
#[derive(Debug, Deserialize)]
pub struct SourceStatus {
    pub id: u32,
    pub name: String,
    pub base_url: String,
    pub last_scraped_at: Option<String>,
    /// When the last run that succeeded finished. Empty until the run log has one.
    pub last_ok_at: Option<String>,
    pub last_run_ok: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct RunRow {
    pub source_id: u32,
    pub trigger: String,
    pub finished_at: String,
    pub ok: u32,
    pub summary: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ViewCount {
    pub kind: String,
    pub week: u32,
    pub month: u32,
}

pub struct Status {
    pub sources: Vec<SourceStatus>,
    pub runs: Vec<RunRow>,
    pub views: Vec<ViewCount>,
}

pub async fn status(db: &D1Database) -> Result<Status> {
    let results = db
        .batch(vec![
            db.prepare(
                "SELECT s.id, s.name, s.base_url, s.last_scraped_at,
                        (SELECT MAX(finished_at) FROM ingest_runs r WHERE r.source_id = s.id AND r.ok = 1) AS last_ok_at,
                        (SELECT ok FROM ingest_runs r WHERE r.source_id = s.id ORDER BY r.id DESC LIMIT 1) AS last_run_ok
                 FROM sources s ORDER BY s.id",
            ),
            db.prepare("SELECT source_id, trigger, finished_at, ok, summary FROM ingest_runs ORDER BY id DESC LIMIT 15"),
            db.prepare(
                "SELECT kind, COALESCE(SUM(CASE WHEN day >= date('now', '+330 minutes', '-6 days') THEN n END), 0) AS week, SUM(n) AS month
                 FROM page_views WHERE day >= date('now', '+330 minutes', '-29 days') GROUP BY kind ORDER BY month DESC",
            ),
        ])
        .await?;
    Ok(Status { sources: results[0].results()?, runs: results[1].results()?, views: results[2].results()? })
}

/// One contractor across the sources.
#[derive(Debug, Deserialize)]
pub struct ContractorRow {
    pub key: String,
    /// The name as PWD writes it, which has spaces; KIIFB's dashboard runs names together.
    pub pwd_name: Option<String>,
    pub kiifb_name: Option<String>,
    pub kiifb_works: u32,
    pub pwd_works: u32,
    pub contract_value: Option<i64>,
}

const CONTRACTOR_ROWS: &str = "SELECT key, MAX(pwd_name) AS pwd_name, MAX(kiifb_name) AS kiifb_name, SUM(kiifb) AS kiifb_works,
        SUM(pwd) AS pwd_works, SUM(value) AS contract_value
    FROM (
      SELECT contractor_key AS key, NULL AS pwd_name, contractor_name AS kiifb_name, 1 AS kiifb, 0 AS pwd, contract_amount AS value
        FROM works WHERE contractor_key IS NOT NULL
      UNION ALL
      SELECT contractor_key, contractor, NULL, 0, 1, NULL FROM liability_works
       WHERE contractor_key IS NOT NULL AND missing_since IS NULL
    )";
pub const CONTRACTOR_PAGE_SIZE: u32 = 60;

/// Contractors by number of works, optionally narrowed by a search on the name.
pub async fn contractors(db: &D1Database, q: &str, page: u32) -> Result<(Vec<ContractorRow>, u32)> {
    let like = format!("%{}%", q.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    const MATCHING: &str = "(?1 = '%%' OR pwd_name LIKE ?1 ESCAPE '\\' OR kiifb_name LIKE ?1 ESCAPE '\\' OR key LIKE ?1 ESCAPE '\\')";
    let results = db
        .batch(vec![
            query!(
                db,
                &format!(
                    "{CONTRACTOR_ROWS} WHERE {MATCHING} GROUP BY key
                     ORDER BY SUM(kiifb) + SUM(pwd) DESC, key LIMIT ?2 OFFSET ?3"
                ),
                like,
                CONTRACTOR_PAGE_SIZE,
                page.saturating_sub(1) * CONTRACTOR_PAGE_SIZE,
            )?,
            query!(db, &format!("SELECT COUNT(*) AS n FROM (SELECT key FROM ({}) WHERE {MATCHING} GROUP BY key)", inner_contractors()), like)?,
        ])
        .await?;
    Ok((results[0].results()?, results[1].results::<Count>()?.into_iter().next().map(|row| row.n).unwrap_or(0)))
}

/// The UNION inside `CONTRACTOR_ROWS`, for counting.
fn inner_contractors() -> &'static str {
    let from = CONTRACTOR_ROWS.find("SELECT contractor_key AS key").unwrap_or(0);
    CONTRACTOR_ROWS[from..].trim_end().trim_end_matches(')')
}

/// A dashboard work by one contractor.
#[derive(Debug, Deserialize)]
pub struct ContractorWork {
    pub code: String,
    pub title_en: String,
    pub road_name: Option<String>,
    pub contractor_name: String,
    pub contract_amount: Option<i64>,
    pub paid_amount: Option<i64>,
    pub scheduled_end: Option<String>,
    pub status: Option<String>,
    pub flag_count: u32,
}

pub struct ContractorPage {
    pub works: Vec<ContractorWork>,
    pub liability: Vec<LiabilityRow>,
}

pub async fn contractor(db: &D1Database, key: &str) -> Result<ContractorPage> {
    let results = db
        .batch(vec![
            query!(
                db,
                "SELECT p.code, p.title_en, w.road_name, w.contractor_name, w.contract_amount, w.paid_amount, w.scheduled_end, w.status, p.flag_count
                 FROM works w JOIN projects p ON p.id = w.project_id WHERE w.contractor_key = ?1
                 ORDER BY COALESCE(w.contract_amount, 0) DESC LIMIT 200",
                key,
            )?,
            query!(
                db,
                &format!("SELECT {LIABILITY_COLUMNS} FROM liability_works WHERE contractor_key = ?1 AND missing_since IS NULL ORDER BY ends_on LIMIT 300"),
                key,
            )?,
        ])
        .await?;
    Ok(ContractorPage { works: results[0].results()?, liability: results[1].results()? })
}

/// One implementing agency across the two KIIFB sources.
#[derive(Debug, Deserialize)]
pub struct AgencyRow {
    pub key: String,
    pub name: String,
    pub packages: u32,
    pub flagged: u32,
    pub funded: u32,
    pub approved: Option<i64>,
    pub paid: Option<i64>,
    /// Status-page projects whose work table has not been read yet.
    pub unread: u32,
}

const AGENCY_ROWS: &str = "SELECT key, MAX(name) AS name, SUM(packages) AS packages, SUM(flagged) AS flagged, SUM(funded) AS funded,
        SUM(approved) AS approved, SUM(paid) AS paid, SUM(unread) AS unread
    FROM (
      SELECT agency_key AS key, MAX(executing_agency) AS name, COUNT(*) AS packages, SUM(flag_count > 0) AS flagged,
             0 AS funded, NULL AS approved, NULL AS paid, 0 AS unread
        FROM projects WHERE agency_key IS NOT NULL AND missing_since IS NULL GROUP BY 1
      UNION ALL
      SELECT agency_key, MAX(spv), 0, 0, COUNT(*), SUM(approved_amount), SUM(released_amount), SUM(detail_checked_at IS NULL)
        FROM funding_projects WHERE agency_key IS NOT NULL AND missing_since IS NULL GROUP BY 1
    ) GROUP BY key";

pub async fn agencies(db: &D1Database) -> Result<Vec<AgencyRow>> {
    db.prepare(format!("{AGENCY_ROWS} ORDER BY COALESCE(SUM(approved), 0) DESC, SUM(packages) DESC, key")).all().await?.results()
}

pub struct AgencyPage {
    pub row: AgencyRow,
    pub funded: Vec<FundingRow>,
    pub packages: Vec<ListRow>,
}

pub async fn agency(db: &D1Database, key: &str) -> Result<Option<AgencyPage>> {
    let results = db
        .batch(vec![
            query!(db, &format!("SELECT * FROM ({AGENCY_ROWS}) WHERE key = ?1"), key)?,
            query!(
                db,
                &format!(
                    "SELECT f.ref, f.name, f.department, f.spv, f.approved_amount, f.released_amount, f.work_count, f.flag_count,
                            f.detail_checked_at IS NOT NULL AS read,
                            (SELECT COUNT(*) FROM projects p WHERE {FUNDING_JOIN}) AS packages
                     FROM funding_projects f WHERE f.agency_key = ?1 AND f.missing_since IS NULL
                     ORDER BY COALESCE(f.approved_amount, 0) DESC, f.name LIMIT 60"
                ),
                key,
            )?,
            query!(
                db,
                &format!(
                    "SELECT {LIST_COLUMNS} FROM projects p WHERE p.agency_key = ?1 AND p.missing_since IS NULL
                     ORDER BY p.flag_count DESC, p.headline_amount DESC, p.code LIMIT 12"
                ),
                key,
            )?,
        ])
        .await?;
    let Some(row) = results[0].results::<AgencyRow>()?.into_iter().next() else { return Ok(None) };
    Ok(Some(AgencyPage { row, funded: results[1].results()?, packages: results[2].results()? }))
}
