//! Nightly read of KIIFB's project status page: what was approved and what has been released.
//!
//! One request lists the district's projects. A project's work table costs one more request, so
//! those are read only for projects that are new or whose figures moved, plus a few of the
//! longest-unchecked ones each night, one at a time with a pause in between.

use std::collections::HashMap;
use std::time::Duration;

use kanakku_core::kiifb_status::{self as status, FundedProject, MapGroup};
use kanakku_core::Date;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::d1::{D1Database, D1PreparedStatement};
use worker::{query, Delay, Env, Error, Fetch, Headers, HttpMetadata, Method, Request, RequestInit, Result};

use crate::ingest::{hex, run_batches, user_agent};

const SOURCE_ID: u32 = 2;
/// Work tables read in one run when nothing forces more.
pub const DEFAULT_DETAILS: usize = 150;
/// Unchanged projects re-read each night, oldest first, so a change in a work table is not missed for long.
const ROTATION: usize = 10;
const PAUSE: Duration = Duration::from_millis(1000);
const FLUSH_EVERY: usize = 15;
const ID: &str = "(SELECT id FROM funding_projects WHERE ref = ?1)";

#[derive(Debug, Default, Serialize)]
pub struct Report {
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
    #[serde(rename = "ref")]
    reference: String,
    record_json: String,
    missing_since: Option<String>,
    detail_checked_at: Option<String>,
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
struct RecordRow {
    record_json: String,
}

pub async fn run(env: &Env, max_details: usize) -> Result<Report> {
    let district_name = env.var("PILOT_DISTRICT")?.to_string();
    let district = status::district_id(&district_name)
        .ok_or_else(|| Error::RustError(format!("{district_name} is not a district on the KIIFB status page")))?;
    let db = env.d1("DB")?;
    let agent = user_agent(env);

    let page = post(&agent, district, None).await?;
    let listed = status::parse_list(&page).map_err(|e| Error::RustError(e.to_string()))?;

    let now_ms = worker::Date::now().as_millis() as i64;
    let now_iso: String = worker::js_sys::Date::new_0().to_iso_string().into();
    let today_iso = Date::from_unix_ms_ist(now_ms).to_iso();
    let mut report = Report { list_bytes: page.len(), projects: listed.len(), ..Report::default() };

    // Hash the parsed rows, not the page, so a change in the page's chrome is not a new snapshot.
    let sha256 = hex(&Sha256::digest(serde_json::to_vec(&listed).map_err(|e| Error::RustError(e.to_string()))?));
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
            store(env, &db, &key, status::SOURCE_URL, &sha256, page.clone().into_bytes(), &now_iso).await?
        }
    };

    let stored_parser = query!(&db, "SELECT parser_version FROM sources WHERE id = ?1", SOURCE_ID)?
        .first::<SourceRow>(None)
        .await?
        .map(|row| row.parser_version)
        .unwrap_or(0);
    // A re-read by a newer parser is not a change at the source, so it is not recorded as one.
    let reparse = stored_parser != 0 && stored_parser != status::PARSER_VERSION;

    let existing: HashMap<String, ExistingRow> = db
        .prepare("SELECT ref, record_json, missing_since, detail_checked_at FROM funding_projects")
        .all()
        .await?
        .results::<ExistingRow>()?
        .into_iter()
        .map(|row| (row.reference.clone(), row))
        .collect();

    // Pass 1: the list. Works are carried over from the stored record until their table is re-read.
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
                        changed_on = CASE WHEN ?8 THEN ?9 ELSE changed_on END
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

    report.joined_to_map = join(&db, &records).await?;

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

/// Joins status projects to the map dashboard's sub-projects, and their works to its packages.
/// Recomputed in full each run: either side may have changed.
pub async fn join(db: &D1Database, records: &HashMap<String, FundedProject>) -> Result<usize> {
    let packages = db
        .prepare(
            "SELECT code, title_en, department, executing_agency, estimated_amount, sub_project_code
             FROM projects WHERE estimated_amount IS NOT NULL AND missing_since IS NULL ORDER BY code",
        )
        .all()
        .await?
        .results::<PackageRow>()?;
    let mut groups: Vec<MapGroup> = Vec::new();
    for row in packages {
        let key = row.sub_project_code.clone().unwrap_or_else(|| row.code.clone());
        match groups.iter_mut().find(|g| g.key == key && g.estimate == row.estimated_amount) {
            Some(group) => group.packages.push((row.code, row.title_en)),
            None => groups.push(MapGroup {
                key,
                estimate: row.estimated_amount,
                department: row.department,
                agency: row.executing_agency,
                packages: vec![(row.code, row.title_en)],
            }),
        }
    }

    // Join against everything stored, not only what this run touched.
    let mut funded: Vec<FundedProject> = db
        .prepare("SELECT record_json FROM funding_projects WHERE missing_since IS NULL")
        .all()
        .await?
        .results::<RecordRow>()?
        .into_iter()
        .filter_map(|row| serde_json::from_str(&row.record_json).ok())
        .collect();
    for project in &mut funded {
        if let Some(fresh) = records.get(&project.reference) {
            *project = fresh.clone();
        }
    }

    let links = status::link(&funded, &groups);
    let mut statements = vec![
        db.prepare("UPDATE funding_projects SET group_key = NULL, match_basis = NULL WHERE group_key IS NOT NULL"),
        db.prepare("UPDATE funding_works SET project_code = NULL WHERE project_code IS NOT NULL"),
    ];
    for project in &funded {
        let Some((key, basis)) = links.get(&project.reference) else { continue };
        statements.push(query!(
            db,
            "UPDATE funding_projects SET group_key = ?2, match_basis = ?3 WHERE ref = ?1",
            project.reference,
            key,
            basis.as_str(),
        )?);
        let Some(group) = groups.iter().find(|g| &g.key == key && Some(g.estimate) == project.approved) else { continue };
        for (i, code) in status::link_works(project, group).into_iter().enumerate() {
            if let Some(code) = code {
                statements.push(query!(
                    db,
                    &format!("UPDATE funding_works SET project_code = ?3 WHERE funding_project_id = {ID} AND seq = ?2"),
                    project.reference,
                    i + 1,
                    code,
                )?);
            }
        }
    }
    run_batches(db, statements).await?;
    Ok(links.len())
}

fn upsert(db: &D1Database, p: &FundedProject, snapshot_id: i64, today: &str, source_changed: bool) -> Result<D1PreparedStatement> {
    query!(
        db,
        "INSERT INTO funding_projects (ref, name, department, spv, main_project, approved_amount, released_amount, status,
                                       work_count, over_paid_works, first_seen_on, changed_on, snapshot_id, record_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?12, ?13)
         ON CONFLICT(ref) DO UPDATE SET
           name = excluded.name, department = excluded.department, spv = excluded.spv,
           approved_amount = excluded.approved_amount, released_amount = excluded.released_amount, status = excluded.status,
           changed_on = CASE WHEN ?14 THEN excluded.changed_on ELSE funding_projects.changed_on END,
           snapshot_id = excluded.snapshot_id, missing_since = NULL, record_json = excluded.record_json",
        p.reference,
        p.name,
        p.department,
        p.spv,
        p.main_project,
        p.approved,
        p.released,
        p.status,
        p.works.len(),
        p.works.iter().filter(|w| w.paid_exceeds_approved()).count(),
        today,
        snapshot_id,
        serde_json::to_string(p).map_err(|e| Error::RustError(e.to_string()))?,
        source_changed,
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

/// Submits the public form, for the district list or for one selected project.
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
