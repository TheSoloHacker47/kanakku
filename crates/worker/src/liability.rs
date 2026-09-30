//! Weekly read of the Kerala PWD defect-liability list for the pilot district's divisions.
//!
//! One request per wing finds the divisions; then each of the district's divisions is read page
//! by page through the site's own pager links, one request at a time with a pause in between.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use kanakku_core::pwd_dlp::{self as dlp, LiabilityWork};
use kanakku_core::Date;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::d1::D1Database;
use worker::{query, Delay, Env, Error, Fetch, Headers, HttpMetadata, Request, RequestInit, Result};

use crate::ingest::{hex, run_batches, user_agent};

const SOURCE_ID: u32 = 3;
/// The list changes slowly, so the nightly trigger only reads it when the last read is this old.
const MIN_AGE_MS: i64 = 6 * 24 * 60 * 60 * 1000 + 12 * 60 * 60 * 1000;
const PAUSE: Duration = Duration::from_millis(1000);
/// A division with more pages than this means the pager was misread.
const MAX_PAGES: u32 = 40;

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub skipped: bool,
    pub requests: usize,
    pub divisions: Vec<String>,
    pub works: usize,
    pub new_snapshot: bool,
    pub added: usize,
    pub unchanged: usize,
    pub went_missing: usize,
    pub returned: usize,
    pub with_amount: usize,
    pub amounts_updated: usize,
}

#[derive(Deserialize)]
struct SnapshotRow {
    id: i64,
    sha256: String,
}

#[derive(Deserialize)]
struct SourceRow {
    last_scraped_at: Option<String>,
}

#[derive(Deserialize)]
struct ExistingRow {
    #[serde(rename = "ref")]
    reference: String,
    missing_since: Option<String>,
    agreed_amount: Option<i64>,
}

/// Runs one read. Without `force` it does nothing if the list was read within the last week.
pub async fn run(env: &Env, force: bool) -> Result<Report> {
    let district = env.var("PILOT_DISTRICT")?.to_string();
    let db = env.d1("DB")?;
    let now_ms = worker::Date::now().as_millis() as i64;
    let now_iso: String = worker::js_sys::Date::new_0().to_iso_string().into();
    let today_iso = Date::from_unix_ms_ist(now_ms).to_iso();
    let mut report = Report::default();

    if !force {
        let last = query!(&db, "SELECT last_scraped_at FROM sources WHERE id = ?1", SOURCE_ID)?
            .first::<SourceRow>(None)
            .await?
            .and_then(|row| row.last_scraped_at);
        let last_ms = last.map(|at| worker::js_sys::Date::new(&at.into()).get_time() as i64);
        if last_ms.is_some_and(|at| now_ms - at < MIN_AGE_MS) {
            report.skipped = true;
            return Ok(report);
        }
    }

    let agent = user_agent(env);
    let mut works: Vec<LiabilityWork> = Vec::new();
    for (wing, wing_name) in dlp::WINGS {
        if report.requests > 0 {
            Delay::from(PAUSE).await;
        }
        let first = get(&agent, &dlp::listing_url(wing, None, 1)).await?;
        report.requests += 1;
        for division in dlp::divisions(&first).into_iter().filter(|d| dlp::in_district(d, &district)) {
            let mut page = 1;
            let mut last_page = 1;
            while page <= last_page.min(MAX_PAGES) {
                Delay::from(PAUSE).await;
                let body = get(&agent, &dlp::listing_url(wing, Some(&division), page)).await?;
                report.requests += 1;
                let listing = dlp::parse_listing(&body, wing_name).map_err(|e| Error::RustError(e.to_string()))?;
                last_page = listing.last_page;
                // The filter is trusted only as far as the rows agree with it.
                works.extend(listing.works.into_iter().filter(|w| w.division.as_deref().is_some_and(|d| dlp::in_district(d, &district))));
                page += 1;
            }
            report.divisions.push(division);
        }
    }
    if works.is_empty() {
        // Nothing at all means the pages changed shape, not that every liability ended at once.
        return Err(Error::RustError(format!("no {district} works in the PWD liability list; refusing to ingest")));
    }

    // PWD repeats some works; keep the first row, and an amount from whichever row carries one.
    let mut seen = HashSet::new();
    let mut keyed: Vec<(String, LiabilityWork)> = Vec::new();
    for work in works {
        let reference = hex(&Sha256::digest(work.identity().as_bytes()))[..16].to_string();
        if seen.insert(reference.clone()) {
            keyed.push((reference, work));
        } else if let Some(kept) = keyed.iter_mut().find(|(r, _)| *r == reference) {
            kept.1.agreed_amount = kept.1.agreed_amount.or(work.agreed_amount);
        }
    }
    report.with_amount = keyed.iter().filter(|(_, w)| w.agreed_amount.is_some()).count();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    report.works = keyed.len();

    // What we keep as evidence is our extract of the rows, because the pages print phone numbers.
    let extract = extract(&keyed, &now_iso);
    let sha256 = hex(&Sha256::digest(
        serde_json::to_vec(&keyed.iter().map(|(_, w)| w).collect::<Vec<_>>()).map_err(|e| Error::RustError(e.to_string()))?,
    ));
    let latest = query!(&db, "SELECT id, sha256 FROM snapshots WHERE source_id = ?1 ORDER BY id DESC LIMIT 1", SOURCE_ID)?
        .first::<SnapshotRow>(None)
        .await?;
    let snapshot_id = match latest {
        Some(row) if row.sha256 == sha256 => row.id,
        _ => {
            report.new_snapshot = true;
            let key = format!("snapshots/pwd-dlp/{today_iso}-{}.tsv", &sha256[..16]);
            let bytes = extract.len();
            env.bucket("BUCKET")?
                .put(key.as_str(), extract.into_bytes())
                .http_metadata(HttpMetadata { content_type: Some("text/tab-separated-values; charset=utf-8".into()), ..HttpMetadata::default() })
                .execute()
                .await?;
            query!(
                &db,
                "INSERT INTO snapshots (source_id, url, r2_key, sha256, bytes, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id, sha256",
                SOURCE_ID,
                dlp::SOURCE_URL,
                key,
                sha256,
                bytes,
                now_iso,
            )?
            .first::<SnapshotRow>(None)
            .await?
            .map(|row| row.id)
            .ok_or_else(|| Error::RustError("snapshot insert returned no row".into()))?
        }
    };

    let existing: HashMap<String, ExistingRow> = db
        .prepare("SELECT ref, missing_since, agreed_amount FROM liability_works")
        .all()
        .await?
        .results::<ExistingRow>()?
        .into_iter()
        .map(|row| (row.reference.clone(), row))
        .collect();

    let mut statements = Vec::new();
    for (reference, work) in &keyed {
        match existing.get(reference) {
            Some(row) => {
                report.unchanged += 1;
                if row.missing_since.is_some() {
                    report.returned += 1;
                    statements.push(query!(&db, "UPDATE liability_works SET missing_since = NULL WHERE ref = ?1", reference)?);
                }
                if row.agreed_amount != work.agreed_amount {
                    report.amounts_updated += 1;
                    statements.push(query!(&db, "UPDATE liability_works SET agreed_amount = ?2 WHERE ref = ?1", reference, work.agreed_amount)?);
                }
            }
            None => {
                report.added += 1;
                statements.push(query!(
                    &db,
                    "INSERT INTO liability_works (ref, wing, name, contractor, contractor_key, starts_on, ends_on, division, subdivision,
                                                  first_seen_on, snapshot_id, agreed_amount)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    reference,
                    work.wing,
                    work.name,
                    work.contractor,
                    work.contractor.as_deref().map(dlp::contractor_key).filter(|key| !key.is_empty()),
                    work.starts_on.map(Date::to_iso),
                    work.ends_on.map(Date::to_iso),
                    work.division,
                    work.subdivision,
                    today_iso,
                    snapshot_id,
                    work.agreed_amount,
                )?);
            }
        }
    }
    for row in existing.values() {
        if row.missing_since.is_none() && !seen.contains(&row.reference) {
            report.went_missing += 1;
            statements.push(query!(&db, "UPDATE liability_works SET missing_since = ?1 WHERE ref = ?2", today_iso, row.reference)?);
        }
    }
    run_batches(&db, statements).await?;
    touch(&db, &now_iso).await?;
    Ok(report)
}

async fn touch(db: &D1Database, now_iso: &str) -> Result<()> {
    query!(db, "UPDATE sources SET last_scraped_at = ?1, parser_version = 1 WHERE id = ?2", now_iso, SOURCE_ID)?.run().await?;
    Ok(())
}

/// The rows we read, as tab-separated text. This is the stored copy people can download.
fn extract(works: &[(String, LiabilityWork)], now_iso: &str) -> String {
    let clean = |s: Option<&str>| s.unwrap_or("").replace(['\t', '\n', '\r'], " ");
    let mut out = format!(
        "# Extract of the Kerala PWD defect-liability list, read {now_iso} from {}\n# Contact numbers printed on the source pages are left out.\n# agreed_amount is in the pages' markup but commented out there; whole rupees.\nwing\twork\tcontractor\tagreed_amount\tliability_starts\tliability_ends\tdivision\tsubdivision\n",
        dlp::SOURCE_URL
    );
    for (_, w) in works {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            w.wing,
            clean(Some(&w.name)),
            clean(w.contractor.as_deref()),
            w.agreed_amount.map(|a| a.to_string()).unwrap_or_default(),
            w.starts_on.map(Date::to_iso).unwrap_or_default(),
            w.ends_on.map(Date::to_iso).unwrap_or_default(),
            clean(w.division.as_deref()),
            clean(w.subdivision.as_deref()),
        ));
    }
    out
}

async fn get(agent: &str, url: &str) -> Result<String> {
    let headers = Headers::new();
    headers.set("User-Agent", agent)?;
    let mut init = RequestInit::new();
    init.with_headers(headers);
    let mut response = Fetch::Request(Request::new_with_init(url, &init)?).send().await?;
    if response.status_code() != 200 {
        return Err(Error::RustError(format!("PWD liability list answered {}", response.status_code())));
    }
    Ok(String::from_utf8_lossy(&response.bytes().await?).into_owned())
}
