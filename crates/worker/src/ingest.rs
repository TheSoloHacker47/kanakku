//! Nightly ingest: fetch the KIIFB dashboard, keep a snapshot, upsert what changed, recompute flags.

use std::collections::HashMap;

use kanakku_core::changes::diff;
use kanakku_core::flags::{self, FlagKind, History, OpenFlag};
use kanakku_core::kiifb;
use kanakku_core::model::Project;
use kanakku_core::Date;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::d1::{D1Database, D1PreparedStatement};
use worker::{query, Env, Error, Fetch, Headers, HttpMetadata, Request, RequestInit, Result};

const SOURCE_ID: u32 = 1;
const BATCH_SIZE: usize = 80;

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub page_bytes: usize,
    pub sha256: String,
    pub new_snapshot: bool,
    pub projects: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub went_missing: usize,
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
struct ExistingRow {
    id: i64,
    code: String,
    record_json: String,
    missing_since: Option<String>,
}

#[derive(Deserialize)]
struct OpenFlagRow {
    id: i64,
    project_id: i64,
    #[serde(rename = "type")]
    kind: String,
    work_ref: Option<String>,
    value_json: String,
}

/// Runs one ingest. `pushed` is a copy of the dashboard page supplied by the caller;
/// without it the Worker fetches the page itself.
pub async fn run(env: &Env, pushed: Option<Vec<u8>>) -> Result<Report> {
    let district = env.var("PILOT_DISTRICT")?.to_string();
    let db = env.d1("DB")?;

    let page = match pushed {
        Some(page) => page,
        None => fetch_page(env).await?,
    };
    let parsed = kiifb::parse(&page, &district).map_err(|e| Error::RustError(e.to_string()))?;
    if parsed.projects.is_empty() {
        // An empty district means the page changed shape, not that every project vanished.
        return Err(Error::RustError(format!("no {district} projects in the KIIFB page; refusing to ingest")));
    }

    let mut hasher = Sha256::new();
    for range in parsed.segments.clone() {
        hasher.update(&page[range]);
    }
    let sha256 = hex(&hasher.finalize());

    let now_ms = worker::Date::now().as_millis() as i64;
    let now_iso: String = worker::js_sys::Date::new_0().to_iso_string().into();
    let today = Date::from_unix_ms_ist(now_ms);
    let today_iso = today.to_iso();

    let mut report = Report { page_bytes: page.len(), sha256: sha256.clone(), projects: parsed.projects.len(), ..Report::default() };

    let latest = query!(&db, "SELECT id, sha256 FROM snapshots WHERE source_id = ?1 ORDER BY id DESC LIMIT 1", SOURCE_ID)?
        .first::<SnapshotRow>(None)
        .await?;

    let snapshot_id = match latest {
        Some(row) if row.sha256 == sha256 => row.id,
        _ => {
            let key = format!("snapshots/kiifb/{today_iso}-{}.html", &sha256[..16]);
            let bytes = page.len();
            env.bucket("BUCKET")?
                .put(key.as_str(), page)
                .http_metadata(HttpMetadata { content_type: Some("text/html; charset=utf-8".into()), ..HttpMetadata::default() })
                .execute()
                .await?;
            let row = query!(
                &db,
                "INSERT INTO snapshots (source_id, url, r2_key, sha256, bytes, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id, sha256",
                SOURCE_ID,
                kiifb::SOURCE_URL,
                key,
                sha256,
                bytes,
                now_iso,
            )?
            .first::<SnapshotRow>(None)
            .await?
            .ok_or_else(|| Error::RustError("snapshot insert returned no row".into()))?;
            report.new_snapshot = true;
            row.id
        }
    };

    if report.new_snapshot {
        upsert_projects(&db, &parsed.projects, snapshot_id, &today_iso, &mut report).await?;
    } else {
        report.unchanged = parsed.projects.len();
    }

    recompute_flags(&db, &parsed.projects, snapshot_id, today, &now_iso, &mut report).await?;

    query!(&db, "UPDATE sources SET last_scraped_at = ?1 WHERE id = ?2", now_iso, SOURCE_ID)?.run().await?;
    Ok(report)
}

async fn fetch_page(env: &Env) -> Result<Vec<u8>> {
    let contact = env.var("CONTACT").map(|v| v.to_string()).unwrap_or_default();
    let agent = if contact.is_empty() {
        "KanakkuBot/0.1 (public project accountability; one request a day)".to_string()
    } else {
        format!("KanakkuBot/0.1 (public project accountability; one request a day; {contact})")
    };
    let headers = Headers::new();
    headers.set("User-Agent", &agent)?;
    let mut init = RequestInit::new();
    init.with_headers(headers);
    let mut response = Fetch::Request(Request::new_with_init(kiifb::SOURCE_URL, &init)?).send().await?;
    if response.status_code() != 200 {
        return Err(Error::RustError(format!("KIIFB dashboard answered {}", response.status_code())));
    }
    response.bytes().await
}

async fn upsert_projects(
    db: &D1Database,
    projects: &[Project],
    snapshot_id: i64,
    today: &str,
    report: &mut Report,
) -> Result<()> {
    let existing: HashMap<String, ExistingRow> = db
        .prepare("SELECT id, code, record_json, missing_since FROM projects")
        .all()
        .await?
        .results::<ExistingRow>()?
        .into_iter()
        .map(|row| (row.code.clone(), row))
        .collect();

    let mut statements = Vec::new();

    for project in projects {
        let record = serde_json::to_string(project).map_err(|e| Error::RustError(e.to_string()))?;
        let code = &project.code;
        match existing.get(code) {
            Some(row) if row.record_json == record => {
                report.unchanged += 1;
                if row.missing_since.is_some() {
                    statements.push(query!(db, "UPDATE projects SET missing_since = NULL WHERE id = ?1", row.id)?);
                }
                continue;
            }
            Some(row) => {
                report.changed += 1;
                if let Ok(before) = serde_json::from_str::<Project>(&row.record_json) {
                    for change in diff(&before, project) {
                        statements.push(query!(
                            db,
                            "INSERT INTO observations (project_id, snapshot_id, field, old_value, new_value, observed_on) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            row.id,
                            snapshot_id,
                            change.field,
                            change.old,
                            change.new,
                            today,
                        )?);
                    }
                }
                for table in ["project_constituencies", "sites", "works"] {
                    statements.push(query!(db, &format!("DELETE FROM {table} WHERE project_id = ?1"), row.id)?);
                }
                statements.push(query!(db, "DELETE FROM projects_fts WHERE rowid = ?1", row.id)?);
            }
            None => report.added += 1,
        }

        statements.push(query!(
            db,
            "INSERT INTO projects (code, title_en, department, sector, executing_agency, district, estimated_amount, expenditure,
                                   works_amount, official_status, first_estimated_amount, first_seen_on, changed_on, snapshot_id, record_json,
                                   sub_project_code, estimate_shared_by, headline_amount)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?7, ?11, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(code) DO UPDATE SET
               title_en = excluded.title_en, department = excluded.department, sector = excluded.sector,
               executing_agency = excluded.executing_agency, district = excluded.district,
               estimated_amount = excluded.estimated_amount, expenditure = excluded.expenditure,
               works_amount = excluded.works_amount, official_status = excluded.official_status,
               first_estimated_amount = COALESCE(projects.first_estimated_amount, excluded.estimated_amount),
               sub_project_code = excluded.sub_project_code, estimate_shared_by = excluded.estimate_shared_by,
               headline_amount = excluded.headline_amount,
               changed_on = excluded.changed_on, missing_since = NULL,
               snapshot_id = excluded.snapshot_id, record_json = excluded.record_json",
            code,
            project.title,
            project.department,
            project.sector,
            project.executing_agency,
            project.district,
            project.estimated_amount,
            project.expenditure,
            project.works_amount(),
            project.status,
            today,
            snapshot_id,
            record,
            project.sub_project_code,
            project.estimate_shared_by.max(1),
            project.headline().map(|(amount, _)| amount).unwrap_or(0),
        )?);
        push_children(db, project, &mut statements)?;
    }

    for row in existing.values() {
        if row.missing_since.is_none() && !projects.iter().any(|p| p.code == row.code) {
            report.went_missing += 1;
            statements.push(query!(db, "UPDATE projects SET missing_since = ?1 WHERE id = ?2", today, row.id)?);
        }
    }

    run_batches(db, statements).await
}

/// Rows that hang off a project: constituencies, pins, works and the search index.
fn push_children(db: &D1Database, project: &Project, statements: &mut Vec<D1PreparedStatement>) -> Result<()> {
    const ID: &str = "(SELECT id FROM projects WHERE code = ?1)";
    let code = &project.code;

    for c in &project.constituencies {
        statements.push(query!(
            db,
            &format!("INSERT INTO project_constituencies (project_id, name, name_ml, mla_name, mla_name_ml) VALUES ({ID}, ?2, ?3, ?4, ?5)"),
            code,
            c.name,
            c.name_ml,
            c.mla_name,
            c.mla_name_ml,
        )?);
    }
    for site in &project.sites {
        statements.push(query!(db, &format!("INSERT INTO sites (project_id, lat, lng) VALUES ({ID}, ?2, ?3)"), code, site.lat, site.lng)?);
    }
    for (i, w) in project.works.iter().enumerate() {
        statements.push(query!(
            db,
            &format!(
                "INSERT INTO works (project_id, seq, work_ref, road_name, spv, contractor_name, as_amount, fs_amount, ts_amount,
                                    tender_amount, loa_amount, contract_amount, paid_amount, paid_contractor, scheduled_start,
                                    scheduled_end, progress_note, physical_pct, financial_pct, status, lat, lng)
                 VALUES ({ID}, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)"
            ),
            code,
            i + 1,
            w.reference(i),
            w.road_name,
            w.spv,
            w.contractor,
            w.as_amount,
            w.fs_amount,
            w.ts_amount,
            w.tender_amount,
            w.loa_amount,
            w.contract_amount,
            w.paid_amount,
            w.paid_contractor,
            w.scheduled_start,
            w.scheduled_end,
            w.progress_note,
            w.physical_pct,
            w.financial_pct,
            w.status,
            w.lat,
            w.lng,
        )?);
    }

    let join = |items: Vec<&str>| items.join(" | ");
    statements.push(query!(
        db,
        &format!("INSERT INTO projects_fts (rowid, code, title, agency, constituencies, contractors, roads) VALUES ({ID}, ?1, ?2, ?3, ?4, ?5, ?6)"),
        code,
        project.title,
        project.executing_agency,
        join(project.constituencies.iter().map(|c| c.name.as_str()).collect()),
        join(project.works.iter().filter_map(|w| w.contractor.as_deref()).collect()),
        join(project.works.iter().filter_map(|w| w.road_name.as_deref()).collect()),
    )?);
    Ok(())
}

async fn recompute_flags(
    db: &D1Database,
    projects: &[Project],
    snapshot_id: i64,
    today: Date,
    now_iso: &str,
    report: &mut Report,
) -> Result<()> {
    #[derive(Deserialize)]
    struct HistoryRow {
        id: i64,
        code: String,
        first_estimated_amount: Option<i64>,
        changed_on: String,
    }

    let results = db
        .batch(vec![
            db.prepare("SELECT id, code, first_estimated_amount, changed_on FROM projects"),
            db.prepare("SELECT id, project_id, type, work_ref, value_json FROM flags WHERE status = 'open'"),
        ])
        .await?;
    let history: HashMap<String, HistoryRow> =
        results[0].results::<HistoryRow>()?.into_iter().map(|row| (row.code.clone(), row)).collect();
    let mut open: HashMap<i64, Vec<OpenFlag>> = HashMap::new();
    for row in results[1].results::<OpenFlagRow>()? {
        let Some(kind) = FlagKind::parse(&row.kind) else { continue };
        open.entry(row.project_id).or_default().push(OpenFlag {
            id: row.id,
            kind,
            work_ref: row.work_ref,
            value: serde_json::from_str(&row.value_json).unwrap_or_default(),
        });
    }

    let today_iso = today.to_iso();
    let mut statements = Vec::new();

    for project in projects {
        let Some(row) = history.get(&project.code) else { continue };
        let computed = flags::evaluate(
            project,
            today,
            &History { first_estimated_amount: row.first_estimated_amount, last_changed: Date::parse_iso(&row.changed_on) },
        );
        report.flags_open += computed.len();
        let plan = flags::reconcile(open.get(&row.id).map(Vec::as_slice).unwrap_or_default(), computed);

        for flag in plan.raise {
            report.flags_raised += 1;
            let value = flag.value.to_string();
            statements.push(query!(
                db,
                "INSERT INTO flags (project_id, work_ref, type, rule_version, value_json, snapshot_id, created_on) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                row.id,
                flag.work_ref,
                flag.kind.as_str(),
                flags::RULES_VERSION,
                value,
                snapshot_id,
                today_iso,
            )?);
            statements.push(audit(db, "flag.raise", &project.code, &serde_json::json!({ "type": flag.kind.as_str(), "work": flag.work_ref, "value": flag.value }), now_iso)?);
        }
        for (id, value) in plan.update {
            statements.push(query!(db, "UPDATE flags SET value_json = ?1, snapshot_id = ?2 WHERE id = ?3", value.to_string(), snapshot_id, id)?);
        }
        for id in plan.clear {
            report.flags_cleared += 1;
            statements.push(query!(db, "UPDATE flags SET status = 'cleared', cleared_on = ?1 WHERE id = ?2", today_iso, id)?);
            statements.push(audit(db, "flag.clear", &project.code, &serde_json::json!({ "flag_id": id }), now_iso)?);
        }
    }

    statements.push(db.prepare(
        "UPDATE projects SET flag_count = (SELECT COUNT(*) FROM flags WHERE flags.project_id = projects.id AND flags.status = 'open')",
    ));
    run_batches(db, statements).await
}

fn audit(db: &D1Database, action: &str, project_code: &str, diff: &serde_json::Value, now_iso: &str) -> Result<D1PreparedStatement> {
    query!(
        db,
        "INSERT INTO audit_log (actor, action, entity, entity_id, diff_json, at) VALUES ('ingest', ?1, 'project', ?2, ?3, ?4)",
        action,
        project_code,
        diff.to_string(),
        now_iso,
    )
}

/// D1 runs each batch as one transaction; chunking keeps every call comfortably small.
async fn run_batches(db: &D1Database, mut statements: Vec<D1PreparedStatement>) -> Result<()> {
    while !statements.is_empty() {
        let rest = statements.split_off(statements.len().min(BATCH_SIZE));
        db.batch(statements).await?;
        statements = rest;
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
    out
}
