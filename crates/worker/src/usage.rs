//! The cost guard: reads this month's account usage from Cloudflare's GraphQL Analytics API and
//! posts to Discord when a metric passes 50%, 80% or 100% of what the plan includes, or is on
//! course to pass it. Each level is announced once a month.
//!
//! Needs `CF_ACCOUNT_ID` (a var) and `CF_ANALYTICS_TOKEN` (a secret: an API token with only
//! "Account Analytics: Read"). Without the token it does nothing.

use kanakku_core::usage::{self, Line, Metric, Usage, PROJECTED};
use kanakku_core::Date;
use serde::Serialize;
use serde_json::{json, Value};
use worker::{query, Env, Fetch, Headers, Method, Request, RequestInit};

const GRAPHQL: &str = "https://api.cloudflare.com/client/v4/graphql";

const QUERY: &str = "query($a: string!, $from: Date!, $to: Date!, $recent: Date!) { viewer { accounts(filter: {accountTag: $a}) {
  workers: workersInvocationsAdaptive(limit: 1, filter: {date_geq: $from, date_leq: $to}) { sum { requests } }
  d1: d1AnalyticsAdaptiveGroups(limit: 1, filter: {date_geq: $from, date_leq: $to}) { sum { rowsRead rowsWritten } }
  r2ops: r2OperationsAdaptiveGroups(limit: 1000, filter: {date_geq: $from, date_leq: $to}) { sum { requests } dimensions { actionType } }
  r2size: r2StorageAdaptiveGroups(limit: 1000, filter: {date_geq: $recent, date_leq: $to}) { max { payloadSize } dimensions { bucketName date } }
} } }";

#[derive(Serialize)]
pub struct Report {
    pub month: String,
    pub day: u32,
    pub days: u32,
    pub lines: Vec<Line>,
    /// Levels reached for the first time this month, which were alerted.
    pub new_alerts: Vec<(Metric, u32)>,
}

/// Today in UTC, which is how the analytics dates and Cloudflare's billing count days.
fn today_utc() -> Date {
    Date::parse_iso(&crate::runs::now_iso()[..10]).expect("ISO date from the runtime")
}

fn month_name(m: u32) -> &'static str {
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"]
        [(m.clamp(1, 12) - 1) as usize]
}

/// This month's usage so far. `None` when the cost guard is not configured.
pub async fn current(env: &Env) -> worker::Result<Option<(Date, u32, Vec<Line>)>> {
    let (Ok(token), Ok(account)) = (env.secret("CF_ANALYTICS_TOKEN"), env.var("CF_ACCOUNT_ID")) else { return Ok(None) };
    let today = today_utc();
    let (y, m, d) = today.ymd();
    let first = Date::from_ymd(y, m, 1).expect("first of the month");
    let next = if m == 12 { Date::from_ymd(y + 1, 1, 1) } else { Date::from_ymd(y, m + 1, 1) }.expect("first of next month");
    let days = next.days_since(first) as u32;

    let body = json!({
        "query": QUERY,
        "variables": { "a": account.to_string(), "from": first.to_iso(), "to": today.to_iso(), "recent": today.plus_days(-2).to_iso() }
    });
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Authorization", &format!("Bearer {token}"))?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers).with_body(Some(body.to_string().into()));
    let mut response = Fetch::Request(Request::new_with_init(GRAPHQL, &init)?).send().await?;
    let value: Value = response.json().await?;
    if let Some(errors) = value.get("errors").filter(|e| !e.is_null() && e.as_array().is_none_or(|a| !a.is_empty())) {
        return Err(worker::Error::RustError(format!("analytics query failed: {errors}")));
    }
    let account = &value["data"]["viewer"]["accounts"][0];
    let num = |v: &Value| v.as_f64().unwrap_or(0.0);

    let mut used = Usage::default();
    used.add(Metric::WorkerRequests, num(&account["workers"][0]["sum"]["requests"]));
    used.add(Metric::D1RowsRead, num(&account["d1"][0]["sum"]["rowsRead"]));
    used.add(Metric::D1RowsWritten, num(&account["d1"][0]["sum"]["rowsWritten"]));
    for row in account["r2ops"].as_array().into_iter().flatten() {
        if let Some(class) = usage::r2_class(row["dimensions"]["actionType"].as_str().unwrap_or_default()) {
            used.add(class, num(&row["sum"]["requests"]));
        }
    }
    // Storage: each bucket's size on its latest reported day, added up.
    let mut latest: std::collections::BTreeMap<String, (String, f64)> = Default::default();
    for row in account["r2size"].as_array().into_iter().flatten() {
        let bucket = row["dimensions"]["bucketName"].as_str().unwrap_or_default().to_string();
        let date = row["dimensions"]["date"].as_str().unwrap_or_default().to_string();
        let size = num(&row["max"]["payloadSize"]);
        let entry = latest.entry(bucket).or_insert((date.clone(), size));
        if date > entry.0 {
            *entry = (date, size);
        }
    }
    used.add(Metric::R2Storage, latest.values().map(|(_, size)| size).sum());

    Ok(Some((today, days, usage::lines(&used, d, days))))
}

/// The daily check. Alerts on levels not yet announced this month.
pub async fn check(env: &Env) -> worker::Result<Option<Report>> {
    let Some((today, days, lines)) = current(env).await? else { return Ok(None) };
    let (y, m, d) = today.ymd();
    let month_key = format!("{y}-{m:02}");

    let db = env.d1("DB")?;
    let mut statements = Vec::new();
    let mut candidates = Vec::new();
    for line in &lines {
        for level in usage::reached(line, d) {
            statements.push(query!(
                &db,
                "INSERT INTO usage_alerts (month, metric, level) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING RETURNING 1 AS ok",
                month_key,
                line.metric.key(),
                level
            )?);
            candidates.push((*line, level));
        }
    }
    let mut hits = Vec::new();
    if !statements.is_empty() {
        #[derive(serde::Deserialize)]
        struct Row {
            #[allow(dead_code)]
            ok: u32,
        }
        for (result, candidate) in db.batch(statements).await?.iter().zip(candidates) {
            if !result.results::<Row>()?.is_empty() {
                hits.push(candidate);
            }
        }
    }
    // A projection warning is redundant once the same metric has actually passed 100%.
    hits.retain(|(line, level)| *level != PROJECTED || line.pct < 100.0);
    if !hits.is_empty() {
        crate::runs::alert(env, &usage::alert_message(&format!("{} {y}", month_name(m)), &hits)).await;
    }
    Ok(Some(Report {
        month: month_key,
        day: d,
        days,
        lines,
        new_alerts: hits.iter().map(|(line, level)| (line.metric, *level)).collect(),
    }))
}
