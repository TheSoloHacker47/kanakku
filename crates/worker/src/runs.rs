//! The run log, failure alerts, visit counts and searches that found nothing.

use kanakku_core::i18n::Lang;
use serde::Serialize;
use worker::{console_error, query, Env, Fetch, Headers, Method, Request, RequestInit};

/// How old a source's last good read may be before it counts as stale: `(source id, hours)`.
/// The dashboard and status page are read nightly, the liability list weekly.
pub const STALE_AFTER_HOURS: [(u32, i64); 3] = [(1, 36), (2, 36), (3, 9 * 24)];

pub fn now_iso() -> String {
    worker::js_sys::Date::new_0().to_iso_string().into()
}

pub fn source_name(source_id: u32) -> &'static str {
    match source_id {
        1 => "KIIFB dashboard",
        2 => "KIIFB project status",
        3 => "PWD liability list",
        _ => "unknown source",
    }
}

/// Writes one run to the log and, if it failed, sends an alert. Never fails itself:
/// a broken log must not hide the ingest result.
pub async fn record<T: Serialize>(env: &Env, source_id: u32, trigger: &str, started_at: &str, result: &worker::Result<T>) {
    let (ok, summary) = match result {
        Ok(report) => (true, serde_json::to_string(report).unwrap_or_default()),
        Err(e) => (false, e.to_string()),
    };
    let summary: String = summary.chars().take(2000).collect();
    let write = async {
        let db = env.d1("DB")?;
        query!(
            &db,
            "INSERT INTO ingest_runs (source_id, trigger, started_at, finished_at, ok, summary) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            source_id,
            trigger,
            started_at,
            now_iso(),
            ok,
            summary,
        )?
        .run()
        .await
    };
    if let Err(e) = write.await {
        console_error!("could not log the run: {e}");
    }
    if !ok {
        alert(env, &format!("Kanakku: the {} read failed ({trigger}). {summary}", source_name(source_id))).await;
    }
}

/// Posts a message to the webhook in the `ALERT_WEBHOOK` secret, when one is set.
/// The body carries the text under the keys Slack, Discord and ntfy-style relays read.
pub async fn alert(env: &Env, message: &str) {
    let Ok(url) = env.secret("ALERT_WEBHOOK").map(|s| s.to_string()) else { return };
    if !url.starts_with("https://") {
        return;
    }
    let send = async {
        let headers = Headers::new();
        headers.set("Content-Type", "application/json")?;
        let body = serde_json::json!({ "text": message, "content": message }).to_string();
        let mut init = RequestInit::new();
        init.with_method(Method::Post).with_headers(headers).with_body(Some(body.into()));
        Fetch::Request(Request::new_with_init(&url, &init)?).send().await
    };
    match send.await {
        Ok(response) if response.status_code() < 300 => {}
        Ok(response) => console_error!("alert webhook answered {}", response.status_code()),
        Err(e) => console_error!("alert webhook failed: {e}"),
    }
}

/// The kind of page a path is, for counting visits. `None` for anything that is not a page.
pub fn view_kind(path: &str) -> Option<(&'static str, Lang)> {
    let (lang, rest) = match path.strip_prefix("/en") {
        Some("") => (Lang::En, "/"),
        Some(rest) if rest.starts_with('/') => (Lang::En, rest),
        _ => (Lang::Ml, path),
    };
    let kind = match rest {
        "/" => "home",
        "/projects" => "projects",
        "/funding" => "funding",
        "/liability" => "liability",
        "/contractors" => "contractors",
        "/agencies" => "agencies",
        "/map" => "map",
        "/methodology" => "methodology",
        "/data" => "data",
        "/status" => "status",
        _ if rest.starts_with("/p/") => "project",
        _ if rest.starts_with("/f/") => "funding_project",
        _ if rest.starts_with("/c/") => "contractor",
        _ if rest.starts_with("/a/") => "agency",
        _ if rest.starts_with("/d/") => "district",
        _ => return None,
    };
    Some((kind, lang))
}

/// Software that fetches pages without a person reading them.
pub fn is_robot(user_agent: &str) -> bool {
    let ua = user_agent.to_ascii_lowercase();
    ua.is_empty() || ["bot", "crawl", "spider", "curl", "wget", "python", "http", "monitor", "preview", "scan", "fetch"].iter().any(|s| ua.contains(s))
}

/// Adds one to today's count for a kind of page.
pub async fn count_view(env: &Env, kind: &'static str, lang: Lang) {
    let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64).to_iso();
    let write = async {
        let db = env.d1("DB")?;
        query!(
            &db,
            "INSERT INTO page_views (day, kind, lang, n) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(day, kind, lang) DO UPDATE SET n = n + 1",
            today,
            kind,
            lang.code(),
        )?
        .run()
        .await
    };
    if let Err(e) = write.await {
        console_error!("could not count a visit: {e}");
    }
}

/// Adds one to today's count for a search that found nothing.
pub async fn count_miss(env: &Env, surface: &'static str, lang: Lang, query: &str) {
    let Some(key) = kanakku_core::search::miss_key(query) else { return };
    let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64).to_iso();
    let write = async {
        let db = env.d1("DB")?;
        query!(
            &db,
            "INSERT INTO search_misses (day, surface, lang, q, n) VALUES (?1, ?2, ?3, ?4, 1)
             ON CONFLICT(day, surface, lang, q) DO UPDATE SET n = n + 1",
            today,
            surface,
            lang.code(),
            key,
        )?
        .run()
        .await
    };
    if let Err(e) = write.await {
        console_error!("could not count a missed search: {e}");
    }
}
