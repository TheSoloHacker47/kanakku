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

/// The longest alert sent, leaving room under Discord's 2,000-character limit.
const ALERT_MAX_CHARS: usize = 1900;

/// Posts a message to the webhook in the `ALERT_WEBHOOK` secret, when one is set.
/// The body carries the text under the keys Slack, Discord and ntfy-style relays read.
pub async fn alert(env: &Env, message: &str) {
    let Ok(url) = env.secret("ALERT_WEBHOOK").map(|s| s.to_string()) else { return };
    if !url.starts_with("https://") {
        return;
    }
    // Discord refuses messages over 2,000 characters, which would lose the alert entirely.
    let message: String = if message.chars().count() > ALERT_MAX_CHARS {
        message.chars().take(ALERT_MAX_CHARS - 1).chain(std::iter::once('…')).collect()
    } else {
        message.to_string()
    };
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
        "/about" => "about",
        "/press" => "press",
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

fn today_iso() -> String {
    kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64).to_iso()
}

/// One page view: today's count for its kind of page, and, when known, for the site the reader
/// came from and for the individual page. All in one round trip.
pub async fn count_view(env: &Env, kind: &'static str, lang: Lang, source: Option<String>, page: Option<String>) {
    let today = today_iso();
    let write = async {
        let db = env.d1("DB")?;
        let mut statements = vec![query!(
            &db,
            "INSERT INTO page_views (day, kind, lang, n) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(day, kind, lang) DO UPDATE SET n = n + 1",
            today,
            kind,
            lang.code(),
        )?];
        for (what, key) in [("source", source), ("page", page)] {
            if let Some(key) = key {
                statements.push(visit_count(&db, &today, what, &key)?);
            }
        }
        db.batch(statements).await
    };
    if let Err(e) = write.await {
        console_error!("could not count a visit: {e}");
    }
}

/// Adds one to today's count for a campaign tag on a link we handed out.
pub async fn count_tag(env: &Env, tag: &str) {
    let today = today_iso();
    let write = async {
        let db = env.d1("DB")?;
        visit_count(&db, &today, "tag", tag)?.run().await
    };
    if let Err(e) = write.await {
        console_error!("could not count a tag: {e}");
    }
}

fn visit_count(db: &worker::d1::D1Database, day: &str, kind: &str, key: &str) -> worker::Result<worker::d1::D1PreparedStatement> {
    query!(
        db,
        "INSERT INTO visit_counts (day, kind, key, n) VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(day, kind, key) DO UPDATE SET n = n + 1",
        day,
        kind,
        key,
    )
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

/// The weekly summary posted to the alert webhook: visits, where readers came from, what they
/// read and what they searched for without finding. Returns the message it sent.
pub async fn digest(env: &Env) -> worker::Result<String> {
    let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64);
    // The seven days before today, and the seven before those.
    let (from, to, prev_from) = (today.plus_days(-7), today.plus_days(-1), today.plus_days(-14));
    let db = env.d1("DB")?;
    let d = crate::db::digest(&db, &from.to_iso(), &to.to_iso(), &prev_from.to_iso()).await?;
    let site = format!("https://{}", env.var("CANONICAL_HOST").map(|v| v.to_string()).unwrap_or_else(|_| "keralakanakku.com".into()));
    let message = digest_message(&site, &from.to_dmy(), &to.to_dmy(), &d);
    alert(env, &message).await;
    Ok(message)
}

fn digest_message(site: &str, from: &str, to: &str, d: &crate::db::Digest) -> String {
    let change = match (d.this_week, d.last_week) {
        (_, 0) => String::new(),
        (now, before) => format!(" ({}{}% on the week before)", if now >= before { "+" } else { "" }, (now as i64 - before as i64) * 100 / before as i64),
    };
    let list = |items: &[crate::db::NamedCount], show: &dyn Fn(&str) -> String| -> String {
        if items.is_empty() {
            return "none yet".to_string();
        }
        items.iter().map(|i| format!("{} ({})", show(&i.key), i.n)).collect::<Vec<_>>().join(", ")
    };
    let mut out = format!("**Kanakku weekly, {from} to {to}**\nPage views: {}{change}\n", d.this_week);
    out.push_str(&format!("**Where readers came from:** {}\n", list(&d.sources, &|k| k.to_string())));
    out.push_str(&format!("**Shared-link tags:** {}\n", list(&d.tags, &|k| k.to_string())));
    // Angle brackets stop Discord from unfurling a preview for every link.
    out.push_str(&format!("**Most read:** {}\n", list(&d.pages, &|k| format!("<{site}{k}>"))));
    out.push_str(&format!("**Searches that found nothing:** {}\n", list(&d.misses, &|k| format!("\"{k}\""))));
    out.push_str(&format!("Status: <{site}/en/status>"));
    out
}
