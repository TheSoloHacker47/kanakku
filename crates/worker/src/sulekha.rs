//! Reads Sulekha's public plan view, a few local bodies a night. Off unless `SULEKHA` names what
//! to read (`Ernakulam:gp`); see `kanakku_core::sulekha` and `docs/sulekha-spike.md`.
//!
//! Manners: one request at a time, two seconds apart, an identifying User-Agent, and a time box
//! well inside the cron limit. Where the read stopped is kept in `crawl_state`, so the next night
//! carries on from the same local body. A finished pass waits `REST_DAYS` before the next.

use std::time::Duration;

use kanakku_core::aspnet::Form;
use kanakku_core::sulekha::{self as s, Cursor, Kind, PlanProject};
use kanakku_core::Date;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::d1::D1Database;
use worker::{console_log, query, Delay, Env, Error, Fetch, Headers, Method, Request, RequestInit, Result};

use crate::ingest::{hex, run_batches, user_agent};
use crate::snapshot;

const PAUSE: Duration = Duration::from_secs(2);
/// Stop starting new local bodies after this long; one more can still take a minute.
const TIME_BOX_MS: f64 = 9.0 * 60_000.0;
const REST_DAYS: i32 = 7;

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub skipped: Option<String>,
    pub requests: u32,
    pub local_bodies: u32,
    pub projects: u32,
    pub finished_pass: bool,
}

/// The cookies and User-Agent of one walk through the form.
struct Session {
    agent: String,
    cookies: Vec<(String, String)>,
    requests: u32,
}

impl Session {
    async fn send(&mut self, body: Option<String>) -> Result<String> {
        if self.requests > 0 {
            Delay::from(PAUSE).await;
        }
        self.requests += 1;
        let headers = Headers::new();
        headers.set("User-Agent", &self.agent)?;
        if !self.cookies.is_empty() {
            let cookie = self.cookies.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join("; ");
            headers.set("Cookie", &cookie)?;
        }
        if body.is_some() {
            headers.set("Content-Type", "application/x-www-form-urlencoded")?;
        }
        let mut init = RequestInit::new();
        init.with_headers(headers);
        if let Some(body) = body {
            init.with_method(Method::Post).with_body(Some(body.into()));
        }
        let mut response = Fetch::Request(Request::new_with_init(s::URL, &init)?).send().await?;
        if response.status_code() != 200 {
            return Err(Error::RustError(format!("Sulekha answered {}", response.status_code())));
        }
        for header in response.headers().get_all("Set-Cookie")? {
            // `name=value; Path=/; HttpOnly`: only the pair is sent back.
            if let Some((name, value)) = header.split(';').next().and_then(|pair| pair.trim().split_once('=')) {
                self.cookies.retain(|(n, _)| n != name);
                self.cookies.push((name.to_string(), value.to_string()));
            }
        }
        Ok(String::from_utf8_lossy(&response.bytes().await?).into_owned())
    }

    /// Clicks `target`/`argument` on `page`, with drop-down `choices`.
    async fn post(&mut self, page: &str, target: &str, argument: &str, choices: &[(&str, &str)]) -> Result<String> {
        let body = Form::parse(page).postback(target, argument, choices);
        self.send(Some(body)).await
    }
}

fn parse_err(e: s::Error) -> Error {
    Error::RustError(e.to_string())
}

#[derive(Deserialize)]
struct StateRow {
    cursor_json: String,
    finished_at: Option<String>,
}

pub async fn run(env: &Env) -> Result<Report> {
    let setting = env.var("SULEKHA").map(|v| v.to_string()).unwrap_or_default();
    let targets = s::targets(&setting);
    if targets.is_empty() {
        return Ok(Report { skipped: Some("SULEKHA is not set".into()), ..Report::default() });
    }
    let year = env.var("SULEKHA_YEAR").map(|v| v.to_string()).unwrap_or_else(|_| s::DEFAULT_YEAR.to_string());
    let db = env.d1("DB")?;
    let started = worker::Date::now().as_millis() as f64;
    let now_iso = crate::runs::now_iso();
    let today = Date::from_unix_ms_ist(started as i64);

    let state = query!(&db, "SELECT cursor_json, finished_at FROM crawl_state WHERE source_id = ?1", s::SOURCE_ID)?.first::<StateRow>(None).await?;
    let mut cursor: Cursor = state.as_ref().and_then(|row| serde_json::from_str(&row.cursor_json).ok()).unwrap_or_default();
    if cursor.finished(targets.len()) {
        let rested = state
            .as_ref()
            .and_then(|row| row.finished_at.as_deref())
            .and_then(Date::from_utc_timestamp_ist)
            .is_none_or(|done| today.days_since(done) >= REST_DAYS);
        if !rested {
            return Ok(Report { skipped: Some("resting after a finished pass".into()), ..Report::default() });
        }
        cursor = Cursor::default();
    }
    if cursor.pass_started.is_empty() {
        cursor.pass_started = now_iso.clone();
    }

    let mut session = Session { agent: user_agent(env), cookies: Vec::new(), requests: 0 };
    let mut report = Report::default();
    while !cursor.finished(targets.len()) && (worker::Date::now().as_millis() as f64 - started) < TIME_BOX_MS {
        let (district, kind) = &targets[cursor.target];
        // Walk to the district's list of local bodies: year, then kind, then the district.
        let start = session.send(None).await?;
        let with_year = session.post(&start, "drpYear", "", &[("drpYear", &year)]).await?;
        let with_kind = session.post(&with_year, "drpType", "", &[("drpYear", &year), ("drpType", kind.form_value())]).await?;
        let districts = s::summaries(&with_kind, "gvState").map_err(parse_err)?;
        let Some(row) = districts.iter().find(|row| row.name.eq_ignore_ascii_case(district)) else {
            return Err(Error::RustError(format!("Sulekha lists no district called {district}")));
        };
        let district_page = session.post(&with_kind, "gvState", &row.select, &[]).await?;
        let local_bodies = s::summaries(&district_page, "gvStat").map_err(parse_err)?;
        if local_bodies.is_empty() {
            return Err(Error::RustError(format!("Sulekha lists no local bodies for {district}; refusing to continue")));
        }

        // Then each local body from where the last night stopped, all its pages.
        while cursor.target < targets.len()
            && targets[cursor.target] == (district.clone(), *kind)
            && (worker::Date::now().as_millis() as f64 - started) < TIME_BOX_MS
        {
            let Some(body) = local_bodies.get(cursor.local_body) else {
                cursor.advance(local_bodies.len());
                continue;
            };
            let mut page = session.post(&district_page, "gvStat", &body.select, &[]).await?;
            let mut projects: Vec<PlanProject> = Vec::new();
            let mut number = 1;
            loop {
                let (rows, pages) = s::projects(&page).map_err(parse_err)?;
                projects.extend(rows);
                number += 1;
                if !pages.contains(&number) {
                    break;
                }
                page = session.post(&page, "gvProjects", &format!("Page${number}"), &[]).await?;
            }
            save(env, &db, &year, district, *kind, &body.name, body.projects, &projects, &now_iso, &today.to_iso()).await?;
            report.local_bodies += 1;
            report.projects += projects.len() as u32;
            cursor.advance(local_bodies.len());
            persist(&db, &cursor, &now_iso, cursor.finished(targets.len())).await?;
        }
    }
    report.requests = session.requests;
    report.finished_pass = cursor.finished(targets.len());
    console_log!("sulekha: {} local bodies, {} projects, {} requests", report.local_bodies, report.projects, report.requests);
    Ok(report)
}

async fn persist(db: &D1Database, cursor: &Cursor, now_iso: &str, finished: bool) -> Result<()> {
    let json = serde_json::to_string(cursor).map_err(|e| Error::RustError(e.to_string()))?;
    query!(
        db,
        "INSERT INTO crawl_state (source_id, cursor_json, updated_at, finished_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (source_id) DO UPDATE SET cursor_json = excluded.cursor_json, updated_at = excluded.updated_at,
           finished_at = COALESCE(excluded.finished_at, crawl_state.finished_at)",
        s::SOURCE_ID,
        json,
        now_iso,
        if finished { Some(now_iso.to_string()) } else { None }
    )?
    .run()
    .await?;
    Ok(())
}

/// Stores one local body's list: the extract as evidence, then the rows.
#[allow(clippy::too_many_arguments)]
async fn save(
    env: &Env,
    db: &D1Database,
    year: &str,
    district: &str,
    kind: Kind,
    name: &str,
    expected: Option<u32>,
    projects: &[PlanProject],
    now_iso: &str,
    today: &str,
) -> Result<()> {
    // A list far shorter than the district table promised is a broken read, not deleted projects.
    if let Some(expected) = expected.filter(|&n| n > 0) {
        if projects.len() * 10 < expected as usize * 7 {
            return Err(Error::RustError(format!("{name}: read {} projects where Sulekha's summary lists {expected}; refusing to store", projects.len())));
        }
    }
    let extract = s::extract(year, district, kind, name, projects);
    let sha256 = hex(&Sha256::digest(extract.as_bytes()));
    let url = format!("{}#year={year}&district={district}&kind={}&body={name}", s::URL, kind.key());
    let snapshot_id = match snapshot::latest(db, s::SOURCE_ID, Some(&url)).await? {
        Some(row) if row.sha256 == sha256 => row.id,
        _ => {
            let slug: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
            let key = format!("snapshots/sulekha/{year}/{}/{}/{slug}-{today}-{}.tsv", district.to_ascii_lowercase(), kind.key(), &sha256[..16]);
            let copy = snapshot::Copy { source_id: s::SOURCE_ID, url: &url, key: &key, content_type: snapshot::TSV, sha256: &sha256, now_iso };
            snapshot::store(env, db, copy, extract.into_bytes()).await?
        }
    };

    #[derive(Deserialize)]
    struct Id {
        id: i64,
    }
    let body = query!(
        db,
        "INSERT INTO local_bodies (district, kind, name, first_seen_on) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (district, kind, name) DO UPDATE SET name = excluded.name RETURNING id",
        district,
        kind.key(),
        name,
        today
    )?
    .first::<Id>(None)
    .await?
    .ok_or_else(|| Error::RustError("local body upsert returned no row".into()))?
    .id;

    let mut statements = Vec::with_capacity(projects.len() + 1);
    for p in projects {
        statements.push(query!(
            db,
            "INSERT INTO plan_projects (local_body_id, year, number, name, planned, spent, first_seen_on, changed_on, snapshot_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)
             ON CONFLICT (local_body_id, year, number) DO UPDATE SET
               changed_on = CASE WHEN plan_projects.name IS NOT excluded.name OR plan_projects.planned IS NOT excluded.planned
                                   OR plan_projects.spent IS NOT excluded.spent THEN excluded.changed_on ELSE plan_projects.changed_on END,
               name = excluded.name, planned = excluded.planned, spent = excluded.spent,
               missing_since = NULL, snapshot_id = excluded.snapshot_id",
            body,
            year,
            p.number,
            p.name,
            p.planned,
            p.spent,
            today,
            snapshot_id
        )?);
    }
    // Rows this read did not see are marked, not deleted.
    statements.push(query!(
        db,
        "UPDATE plan_projects SET missing_since = ?3 WHERE local_body_id = ?1 AND year = ?2 AND missing_since IS NULL AND snapshot_id != ?4",
        body,
        year,
        today,
        snapshot_id
    )?);
    run_batches(db, statements).await
}
