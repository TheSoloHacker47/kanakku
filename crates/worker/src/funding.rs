//! Nightly read of KIIFB's project status page: what was approved and what has been paid.
//!
//! One request per district lists its projects. A project's work table costs one more request, so
//! those are read only for projects that are new or whose figures moved, plus a few of the
//! longest-unchecked ones each night, one at a time with a pause in between.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use kanakku_core::entity::agency_key;
use kanakku_core::flags::{self, FlagKind, OpenFlag};
use kanakku_core::kiifb_status::{self as status, FundedProject, MapGroup};
use kanakku_core::names::ALL_DISTRICTS;
use kanakku_core::Date;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::d1::{D1Database, D1PreparedStatement};
use worker::{query, Delay, Env, Error, Fetch, Headers, HttpMetadata, Method, Request, RequestInit, Result};

use crate::ingest::{hex, run_batches, user_agent};

const SOURCE_ID: u32 = 2;
/// Work tables read in one run when nothing forces more.
/// Kept well inside the limit on requests one run may make: each table is a fetch, a stored copy and a row.
pub const DEFAULT_DETAILS: usize = 120;
/// Unchanged projects re-read each night, oldest first, so a change in a work table is not missed for long.
const ROTATION: usize = 10;
const PAUSE: Duration = Duration::from_millis(1000);
const FLUSH_EVERY: usize = 15;
const ID: &str = "(SELECT id FROM funding_projects WHERE ref = ?1)";

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub districts: usize,
    pub list_bytes: usize,
    pub new_snapshot: bool,
    pub projects: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub went_missing: usize,
    pub details_read: usize,
    pub details_changed: usize,
    pub details_failed: usize,
    /// Work tables still waiting for a later run.
    pub details_pending: usize,
    pub joined_to_map: usize,
    pub flags_raised: usize,
    pub flags_cleared: usize,
    pub flags_open: usize,
}

#[derive(Deserialize)]
struct SnapshotRow {
    id: i64,
    sha256: String,
}

#[derive(Deserialize)]
struct SourceRow {
    parser_version: u32,
}

#[derive(Deserialize)]
struct ExistingRow {
    id: i64,
    #[serde(rename = "ref")]
    reference: String,
    record_json: String,
    missing_since: Option<String>,
    detail_checked_at: Option<String>,
    group_key: Option<String>,
    match_basis: Option<String>,
    agency_key: Option<String>,
}

#[derive(Deserialize)]
struct PackageRow {
    code: String,
    title_en: String,
    department: Option<String>,
    executing_agency: Option<String>,
    estimated_amount: i64,
    sub_project_code: Option<String>,
}

#[derive(Deserialize)]
struct WorkLinkRow {
    #[serde(rename = "ref")]
    reference: String,
    seq: u32,
    project_code: String,
}

#[derive(Deserialize)]
struct OpenFlagRow {
    id: i64,
    #[serde(rename = "ref")]
    reference: String,
    #[serde(rename = "type")]
    kind: String,
    work_ref: Option<String>,
    value_json: String,
}

#[derive(Deserialize)]
struct Count {
    n: u32,
}

pub async fn run(env: &Env, max_details: usize) -> Result<Report> {
    let scope = env.var("PILOT_DISTRICT")?.to_string();
    let districts: Vec<(u8, &'static str)> = if scope == ALL_DISTRICTS {
        status::districts().collect()
    } else {
        let id = status::district_id(&scope)
            .ok_or_else(|| Error::RustError(format!("{scope} is not a district on the KIIFB status page")))?;
        status::districts().filter(|(n, _)| *n == id).collect()
    };
    let db = env.d1("DB")?;
    let agent = user_agent(env);

    // One list per district. A project filed under several districts appears in each list.
    let mut listed: Vec<FundedProject> = Vec::new();
    let mut filed: HashMap<String, Vec<(u8, &'static str)>> = HashMap::new();
    let mut pages = String::new();
    for (n, (id, name)) in districts.iter().enumerate() {
        if n > 0 {
            Delay::from(PAUSE).await;
        }
        let page = post(&agent, *id, None).await?;
        for project in status::parse_list(&page).map_err(|e| Error::RustError(format!("{name}: {e}")))? {
            let under = filed.entry(project.reference.clone()).or_default();
            if under.is_empty() {
                listed.push(project);
            }
            under.push((*id, name));
        }
        pages.push_str(&format!("<!-- KIIFB project status, district: {name} -->\n"));
        pages.push_str(&page);
        pages.push('\n');
    }

    let now_ms = worker::Date::now().as_millis() as i64;
    let now_iso: String = worker::js_sys::Date::new_0().to_iso_string().into();
    let today_iso = Date::from_unix_ms_ist(now_ms).to_iso();
    let mut report = Report { districts: districts.len(), list_bytes: pages.len(), projects: listed.len(), ..Report::default() };

    // Hash the parsed rows, not the pages, so a change in the page's chrome is not a new snapshot.
    let mut filing: Vec<(&String, Vec<&str>)> = filed.iter().map(|(r, d)| (r, d.iter().map(|(_, name)| *name).collect())).collect();
    filing.sort();
    let sha256 = hex(&Sha256::digest(serde_json::to_vec(&(&listed, &filing)).map_err(|e| Error::RustError(e.to_string()))?));
    let latest = query!(
        &db,
        "SELECT id, sha256 FROM snapshots WHERE source_id = ?1 AND url = ?2 ORDER BY id DESC LIMIT 1",
        SOURCE_ID,
        status::SOURCE_URL,
    )?
    .first::<SnapshotRow>(None)
    .await?;
    let snapshot_id = match latest {
        Some(row) if row.sha256 == sha256 => row.id,
        _ => {
            report.new_snapshot = true;
            let key = format!("snapshots/kiifb-status/{today_iso}-{}.html", &sha256[..16]);
            store(env, &db, &key, status::SOURCE_URL, &sha256, pages.into_bytes(), &now_iso).await?
        }
    };

    let stored_parser = query!(&db, "SELECT parser_version FROM sources WHERE id = ?1", SOURCE_ID)?
        .first::<SourceRow>(None)
        .await?
        .map(|row| row.parser_version)
        .unwrap_or(0);
    // A re-read by a newer parser is not a change at the source, so it is not recorded as one.
    let reparse = stored_parser != 0 && stored_parser != status::PARSER_VERSION;

    let mut existing: HashMap<String, ExistingRow> = HashMap::new();
    let mut after = 0i64;
    loop {
        let page = query!(
            &db,
            "SELECT id, ref, record_json, missing_since, detail_checked_at, group_key, match_basis, agency_key
             FROM funding_projects WHERE id > ?1 ORDER BY id LIMIT 500",
            after,
        )?
        .all()
        .await?
        .results::<ExistingRow>()?;
        let Some(last) = page.last() else { break };
        after = last.id;
        existing.extend(page.into_iter().map(|row| (row.reference.clone(), row)));
    }

    // Pass 1: the lists. Works are carried over from the stored record until their table is re-read.
    let mut statements = Vec::new();
    let mut records: HashMap<String, FundedProject> = HashMap::new();
    let mut must_read: Vec<String> = Vec::new();
    let mut can_wait: Vec<(String, String)> = Vec::new();

    for mut project in listed {
        let before = existing.get(&project.reference);
        let stored: Option<FundedProject> = before.and_then(|row| serde_json::from_str(&row.record_json).ok());
        if let Some(stored) = &stored {
            project.main_project = stored.main_project.clone();
            project.works = stored.works.clone();
        }
        let reference = project.reference.clone();

        match (before, &stored) {
            (Some(row), Some(stored)) if *stored == project => {
                report.unchanged += 1;
                if row.missing_since.is_some() {
                    statements.push(query!(&db, "UPDATE funding_projects SET missing_since = NULL WHERE ref = ?1", reference)?);
                }
                match &row.detail_checked_at {
                    Some(at) if !reparse => can_wait.push((at.clone(), reference.clone())),
                    _ => must_read.push(reference.clone()),
                }
            }
            _ => {
                if before.is_some() {
                    report.changed += 1;
                } else {
                    report.added += 1;
                }
                if let Some(stored) = stored.as_ref().filter(|_| !reparse) {
                    for change in status::diff(stored, &project) {
                        statements.push(observation(&db, &reference, snapshot_id, &change.field, change.old, change.new, &today_iso)?);
                    }
                }
                statements.push(upsert(&db, &project, snapshot_id, &today_iso, !reparse)?);
                must_read.push(reference.clone());
            }
        }
        records.insert(reference, project);
    }
    for row in existing.values() {
        if row.missing_since.is_none() && !records.contains_key(&row.reference) {
            report.went_missing += 1;
            statements.push(query!(&db, "UPDATE funding_projects SET missing_since = ?1 WHERE ref = ?2", today_iso, row.reference)?);
        }
    }
    run_batches(&db, std::mem::take(&mut statements)).await?;

    // Which districts each project is filed under. Rewritten only when the lists changed.
    let filed_rows = db.prepare("SELECT COUNT(*) AS n FROM funding_districts").first::<Count>(None).await?.map(|row| row.n).unwrap_or(0);
    if report.new_snapshot || filed_rows == 0 {
        statements.push(db.prepare("DELETE FROM funding_districts"));
        for (reference, under) in &filed {
            for (_, name) in under {
                statements.push(query!(
                    &db,
                    &format!("INSERT OR IGNORE INTO funding_districts (funding_project_id, district) VALUES ({ID}, ?2)"),
                    reference,
                    *name,
                )?);
            }
        }
        run_batches(&db, std::mem::take(&mut statements)).await?;
    }

    // Pass 2: work tables.
    can_wait.sort();
    let mut queue = must_read;
    let forced = queue.len();
    queue.extend(can_wait.into_iter().take(ROTATION).map(|(_, reference)| reference));
    report.details_pending = forced.saturating_sub(max_details);
    queue.truncate(max_details);

    for (n, reference) in queue.iter().enumerate() {
        Delay::from(PAUSE).await;
        let Some(current) = records.get(reference) else { continue };
        let district = filed.get(reference).and_then(|under| under.first()).map(|(id, _)| *id).unwrap_or(0);
        let detail_page = match post(&agent, district, Some(reference)).await {
            Ok(page) => page,
            Err(e) => {
                worker::console_error!("status detail {reference} failed: {e}");
                report.details_failed += 1;
                continue;
            }
        };
        let Ok(detail) = status::parse_detail(&detail_page) else {
            report.details_failed += 1;
            continue;
        };
        report.details_read += 1;

        let mut updated = current.clone();
        updated.main_project = detail.main_project.clone();
        updated.works = detail.works.clone();
        let first_read = existing.get(reference).is_none_or(|row| row.detail_checked_at.is_none());

        if updated == *current && !first_read {
            statements.push(query!(&db, "UPDATE funding_projects SET detail_checked_at = ?1 WHERE ref = ?2", now_iso, reference)?);
        } else {
            report.details_changed += 1;
            let panel_sha = hex(&Sha256::digest(detail.panel.as_bytes()));
            let key = format!("snapshots/kiifb-status/works/{reference}-{today_iso}-{}.html", &panel_sha[..16]);
            let url = format!("{}#{reference}", status::SOURCE_URL);
            let detail_snapshot = store(env, &db, &key, &url, &panel_sha, detail.panel.as_bytes().to_vec(), &now_iso).await?;

            let record_change = !first_read && !reparse;
            if record_change {
                for change in status::diff(current, &updated) {
                    statements.push(observation(&db, reference, detail_snapshot, &change.field, change.old, change.new, &today_iso)?);
                }
            }
            statements.push(query!(
                &db,
                "UPDATE funding_projects SET main_project = ?2, work_count = ?3, over_paid_works = ?4, record_json = ?5,
                        detail_snapshot_id = ?6, detail_checked_at = ?7,
                        changed_on = CASE WHEN ?8 THEN ?9 ELSE changed_on END, released_amount = ?10
                 WHERE ref = ?1",
                reference,
                updated.main_project,
                updated.works.len(),
                updated.works.iter().filter(|w| w.paid_exceeds_approved()).count(),
                serde_json::to_string(&updated).map_err(|e| Error::RustError(e.to_string()))?,
                detail_snapshot,
                now_iso,
                record_change,
                today_iso,
                updated.paid(),
            )?);
            statements.push(query!(&db, &format!("DELETE FROM funding_works WHERE funding_project_id = {ID}"), reference)?);
            for (i, w) in updated.works.iter().enumerate() {
                statements.push(query!(
                    &db,
                    &format!(
                        "INSERT INTO funding_works (funding_project_id, seq, name, spv, approved_amount, paid_amount, status)
                         VALUES ({ID}, ?2, ?3, ?4, ?5, ?6, ?7)"
                    ),
                    reference,
                    i + 1,
                    w.name,
                    w.spv,
                    w.approved,
                    w.paid,
                    w.status,
                )?);
            }
            records.insert(reference.clone(), updated);
        }
        if (n + 1) % FLUSH_EVERY == 0 {
            run_batches(&db, std::mem::take(&mut statements)).await?;
        }
    }
    run_batches(&db, std::mem::take(&mut statements)).await?;

    recompute_flags(&db, &records, snapshot_id, &today_iso, &mut report).await?;
    report.joined_to_map = join(&db, &records, &existing).await?;

    query!(
        &db,
        "UPDATE sources SET last_scraped_at = ?1, parser_version = ?2 WHERE id = ?3",
        now_iso,
        status::PARSER_VERSION,
        SOURCE_ID,
    )?
    .run()
    .await?;
    Ok(report)
}

/// Brings the stored flags in line with what the works say now.
async fn recompute_flags(
    db: &D1Database,
    records: &HashMap<String, FundedProject>,
    snapshot_id: i64,
    today_iso: &str,
    report: &mut Report,
) -> Result<()> {
    let mut open: HashMap<String, Vec<OpenFlag>> = HashMap::new();
    let rows = db
        .prepare(
            "SELECT ff.id, f.ref, ff.type, ff.work_ref, ff.value_json
             FROM funding_flags ff JOIN funding_projects f ON f.id = ff.funding_project_id WHERE ff.status = 'open'",
        )
        .all()
        .await?
        .results::<OpenFlagRow>()?;
    for row in rows {
        let Some(kind) = FlagKind::parse(&row.kind) else { continue };
        open.entry(row.reference).or_default().push(OpenFlag {
            id: row.id,
            kind,
            work_ref: row.work_ref,
            value: serde_json::from_str(&row.value_json).unwrap_or_default(),
        });
    }

    let mut statements = Vec::new();
    for (reference, project) in records {
        let computed = status::flags(project);
        let was_open = open.get(reference).map(Vec::as_slice).unwrap_or_default();
        if computed.is_empty() && was_open.is_empty() {
            continue;
        }
        report.flags_open += computed.len();
        let plan = flags::reconcile(was_open, computed);
        for flag in plan.raise {
            report.flags_raised += 1;
            statements.push(query!(
                db,
                &format!(
                    "INSERT INTO funding_flags (funding_project_id, work_ref, type, rule_version, value_json, snapshot_id, created_on)
                     VALUES ({ID}, ?2, ?3, ?4, ?5, ?6, ?7)"
                ),
                reference,
                flag.work_ref,
                flag.kind.as_str(),
                flags::RULES_VERSION,
                flag.value.to_string(),
                snapshot_id,
                today_iso,
            )?);
        }
        for (id, value) in plan.update {
            statements.push(query!(db, "UPDATE funding_flags SET value_json = ?1, snapshot_id = ?2 WHERE id = ?3", value.to_string(), snapshot_id, id)?);
        }
        for id in plan.clear {
            report.flags_cleared += 1;
            statements.push(query!(db, "UPDATE funding_flags SET status = 'cleared', cleared_on = ?1 WHERE id = ?2", today_iso, id)?);
        }
    }
    if !statements.is_empty() {
        statements.push(db.prepare(
            "UPDATE funding_projects SET flag_count =
               (SELECT COUNT(*) FROM funding_flags ff WHERE ff.funding_project_id = funding_projects.id AND ff.status = 'open')",
        ));
    }
    run_batches(db, statements).await
}

/// Joins status projects to the map dashboard's sub-projects, and their works to its packages.
/// Worked out in full each run, since either side may have changed, but only differences are written.
async fn join(db: &D1Database, records: &HashMap<String, FundedProject>, existing: &HashMap<String, ExistingRow>) -> Result<usize> {
    let packages = db
        .prepare(
            "SELECT code, title_en, department, executing_agency, estimated_amount, sub_project_code
             FROM projects WHERE estimated_amount IS NOT NULL AND missing_since IS NULL ORDER BY code",
        )
        .all()
        .await?
        .results::<PackageRow>()?;
    let mut groups: Vec<MapGroup> = Vec::new();
    let mut index: HashMap<(String, i64), usize> = HashMap::new();
    for row in packages {
        let key = row.sub_project_code.clone().unwrap_or_else(|| row.code.clone());
        match index.get(&(key.clone(), row.estimated_amount)) {
            Some(i) => groups[*i].packages.push((row.code, row.title_en)),
            None => {
                index.insert((key.clone(), row.estimated_amount), groups.len());
                groups.push(MapGroup {
                    key,
                    estimate: row.estimated_amount,
                    department: row.department,
                    agency: row.executing_agency,
                    packages: vec![(row.code, row.title_en)],
                });
            }
        }
    }

    let funded: Vec<FundedProject> = records.values().cloned().collect();
    let links = status::link(&funded, &groups);

    let mut stored_works: HashMap<(String, u32), String> = db
        .prepare(
            "SELECT f.ref, w.seq, w.project_code FROM funding_works w JOIN funding_projects f ON f.id = w.funding_project_id
             WHERE w.project_code IS NOT NULL",
        )
        .all()
        .await?
        .results::<WorkLinkRow>()?
        .into_iter()
        .map(|row| ((row.reference, row.seq), row.project_code))
        .collect();

    let mut statements = Vec::new();
    for project in &funded {
        let reference = &project.reference;
        let before = existing.get(reference);
        let link = links.get(reference);
        let (key, basis) = (link.map(|(key, _)| key.as_str()), link.map(|(_, basis)| basis.as_str()));
        let agency = project.spv.as_deref().map(agency_key).filter(|k| !k.is_empty());
        let unchanged = before.is_some_and(|row| {
            row.group_key.as_deref() == key && row.match_basis.as_deref() == basis && row.agency_key == agency
        });
        if !unchanged {
            statements.push(query!(
                db,
                "UPDATE funding_projects SET group_key = ?2, match_basis = ?3, agency_key = ?4 WHERE ref = ?1",
                reference,
                key,
                basis,
                agency,
            )?);
        }

        let group = key.and_then(|key| index.get(&(key.to_string(), project.approved.unwrap_or(0)))).map(|i| &groups[*i]);
        let wanted = group.map(|group| status::link_works(project, group)).unwrap_or_default();
        for (i, code) in wanted.into_iter().enumerate() {
            let seq = i as u32 + 1;
            let stored = stored_works.remove(&(reference.clone(), seq));
            if stored != code {
                statements.push(query!(
                    db,
                    &format!("UPDATE funding_works SET project_code = ?3 WHERE funding_project_id = {ID} AND seq = ?2"),
                    reference,
                    seq,
                    code,
                )?);
            }
        }
    }
    // Links left over belong to works that are no longer joined.
    let listed: HashSet<&String> = funded.iter().map(|p| &p.reference).collect();
    for ((reference, seq), _) in stored_works.into_iter().filter(|((reference, _), _)| listed.contains(reference)) {
        statements.push(query!(
            db,
            &format!("UPDATE funding_works SET project_code = NULL WHERE funding_project_id = {ID} AND seq = ?2"),
            reference,
            seq,
        )?);
    }
    run_batches(db, statements).await?;
    Ok(links.len())
}

fn upsert(db: &D1Database, p: &FundedProject, snapshot_id: i64, today: &str, source_changed: bool) -> Result<D1PreparedStatement> {
    query!(
        db,
        "INSERT INTO funding_projects (ref, name, department, spv, main_project, approved_amount, released_amount, status,
                                       work_count, over_paid_works, first_seen_on, changed_on, snapshot_id, record_json, released_listed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?12, ?13, ?15)
         ON CONFLICT(ref) DO UPDATE SET
           name = excluded.name, department = excluded.department, spv = excluded.spv,
           approved_amount = excluded.approved_amount, released_amount = excluded.released_amount, status = excluded.status,
           released_listed = excluded.released_listed,
           changed_on = CASE WHEN ?14 THEN excluded.changed_on ELSE funding_projects.changed_on END,
           snapshot_id = excluded.snapshot_id, missing_since = NULL, record_json = excluded.record_json",
        p.reference,
        p.name,
        p.department,
        p.spv,
        p.main_project,
        p.approved,
        p.paid(),
        p.status,
        p.works.len(),
        p.works.iter().filter(|w| w.paid_exceeds_approved()).count(),
        today,
        snapshot_id,
        serde_json::to_string(p).map_err(|e| Error::RustError(e.to_string()))?,
        source_changed,
        p.released,
    )
}

fn observation(
    db: &D1Database,
    reference: &str,
    snapshot_id: i64,
    field: &str,
    old: Option<String>,
    new: Option<String>,
    today: &str,
) -> Result<D1PreparedStatement> {
    query!(
        db,
        &format!(
            "INSERT INTO funding_observations (funding_project_id, snapshot_id, field, old_value, new_value, observed_on)
             VALUES ({ID}, ?2, ?3, ?4, ?5, ?6)"
        ),
        reference,
        snapshot_id,
        field,
        old,
        new,
        today,
    )
}

/// Keeps a copy in R2 and records it. Returns the snapshot id.
async fn store(env: &Env, db: &D1Database, key: &str, url: &str, sha256: &str, body: Vec<u8>, now_iso: &str) -> Result<i64> {
    let bytes = body.len();
    env.bucket("BUCKET")?
        .put(key, body)
        .http_metadata(HttpMetadata { content_type: Some("text/html; charset=utf-8".into()), ..HttpMetadata::default() })
        .execute()
        .await?;
    query!(
        db,
        "INSERT INTO snapshots (source_id, url, r2_key, sha256, bytes, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id, sha256",
        SOURCE_ID,
        url,
        key,
        sha256,
        bytes,
        now_iso,
    )?
    .first::<SnapshotRow>(None)
    .await?
    .map(|row| row.id)
    .ok_or_else(|| Error::RustError("snapshot insert returned no row".into()))
}

/// Submits the public form, for a district's list or for one selected project.
async fn post(agent: &str, district: u8, selected: Option<&str>) -> Result<String> {
    let body = status::form_body(district, selected).ok_or_else(|| Error::RustError("unexpected project reference".into()))?;
    let headers = Headers::new();
    headers.set("User-Agent", agent)?;
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers).with_body(Some(body.into()));
    let mut response = Fetch::Request(Request::new_with_init(status::FORM_URL, &init)?).send().await?;
    if response.status_code() != 200 {
        return Err(Error::RustError(format!("KIIFB status page answered {}", response.status_code())));
    }
    Ok(String::from_utf8_lossy(&response.bytes().await?).into_owned())
}
