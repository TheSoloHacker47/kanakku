//! Kanakku on Cloudflare Workers: server-rendered pages, open data, and the nightly ingest.

use kanakku_core::i18n::Lang;
use kanakku_core::model::Project;
use worker::{
    console_error, console_log, event, Cache, Context, Env, Error, Headers, Method, Request, Response, Result, ScheduleContext,
    ScheduledEvent,
};

mod api;
mod db;
mod district;
mod funding;
mod http;
mod icons;
mod ingest;
mod liability;
mod runs;
mod views;

use http::Policy;

const TILES_PATH: &str = "/tiles/ernakulam.pmtiles";
const TILES_KEY: &str = "tiles/ernakulam.pmtiles";

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let result = match req.method() {
        // Range requests for map tiles go straight to R2; the Cache API cannot store partial responses.
        Method::Get | Method::Head if req.path() == TILES_PATH => tiles(&req, &env).await,
        Method::Get | Method::Head => cached(req, &env, &ctx).await,
        Method::Post => admin(req, &env).await,
        _ => Response::error("Method not allowed", 405),
    };
    result.or_else(|e| {
        console_error!("request failed: {e}");
        Response::error("Something went wrong on our side.", 500)
    })
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    // The sources fail independently; one being down must not stop the others.
    let started = runs::now_iso();
    let result = ingest::run(&env, None).await;
    log_outcome("dashboard", &result);
    runs::record(&env, 1, "cron", &started, &result).await;

    let started = runs::now_iso();
    let result = funding::run(&env, funding::DEFAULT_DETAILS).await;
    log_outcome("status", &result);
    runs::record(&env, 2, "cron", &started, &result).await;

    let started = runs::now_iso();
    let result = liability::run(&env, false).await;
    log_outcome("liability", &result);
    // The liability list is read weekly; a night it is skipped is not a run.
    if !matches!(&result, Ok(report) if report.skipped) {
        runs::record(&env, 3, "cron", &started, &result).await;
    }
}

fn log_outcome<T: serde::Serialize>(name: &str, result: &Result<T>) {
    match result {
        Ok(report) => console_log!("{name} ingest ok: {}", serde_json::to_string(report).unwrap_or_default()),
        Err(e) => console_error!("{name} ingest failed: {e}"),
    }
}

/// Serves a GET from the edge cache when it can, and fills the cache when it cannot.
async fn cached(req: Request, env: &Env, ctx: &Context) -> Result<Response> {
    // The health check must always reflect the present.
    let use_cache = env.var("EDGE_CACHE").map(|v| v.to_string() == "on").unwrap_or(false) && req.path() != "/api/v1/status";
    let cache = Cache::default();

    // Count the visit whether or not the page comes from the cache.
    if req.method() == Method::Get {
        let robot = req.headers().get("User-Agent")?.is_none_or(|ua| runs::is_robot(&ua));
        if let Some((kind, lang)) = runs::view_kind(&req.path()).filter(|_| !robot) {
            let env = env.clone();
            ctx.wait_until(async move { runs::count_view(&env, kind, lang).await });
        }
    }

    if use_cache {
        if let Some(hit) = cache.get(&req, false).await? {
            return Ok(hit);
        }
    }

    let mut response = route(&req, env).await?;
    if response.status_code() != 200 {
        return Ok(response);
    }
    if use_cache {
        let copy = response.cloned()?;
        let key = req.url()?.to_string();
        ctx.wait_until(async move {
            if let Err(e) = cache.put(key, copy).await {
                console_error!("cache put failed: {e}");
            }
        });
    }
    // The cache answers conditional requests itself; this covers a fresh render.
    let etag = response.headers().get("ETag")?;
    if etag.is_some() && etag == req.headers().get("If-None-Match")? {
        let headers = Headers::new();
        for name in ["ETag", "Cache-Control"] {
            if let Some(value) = response.headers().get(name)? {
                headers.set(name, &value)?;
            }
        }
        return Ok(Response::empty()?.with_status(304).with_headers(headers));
    }
    Ok(response)
}

async fn route(req: &Request, env: &Env) -> Result<Response> {
    let path = req.path();
    let (lang, rest) = match path.strip_prefix("/en") {
        Some("") => (Lang::En, "/"),
        Some(rest) if rest.starts_with('/') => (Lang::En, rest),
        _ => (Lang::Ml, path.as_str()),
    };
    let db = env.d1("DB")?;
    let origin = req.url()?.origin().ascii_serialization();
    let origin = origin.as_str();
    let checked = || async { db::last_checked(&db).await };

    match rest {
        // The list used to live at the root; keep old search and filter links working.
        "/" if req.url()?.query().is_some_and(|q| !q.is_empty()) => {
            let mut to = req.url()?;
            to.set_path(&format!("{}/projects", lang.prefix()));
            Response::redirect_with_status(to, 301)
        }
        "/" => http::html(views::home::render(lang, origin, &db::home(&db).await?), 200, Policy::Page),
        "/projects" => {
            let filter = filter_from(req)?;
            let listing = db::list(&db, &filter).await?;
            http::html(views::list::render(lang, origin, &filter, &listing), 200, Policy::Page)
        }
        "/methodology" => http::html(views::pages::methodology(lang, origin, checked().await?.as_deref()), 200, Policy::Page),
        "/data" => http::html(views::pages::data(lang, origin, checked().await?.as_deref()), 200, Policy::Page),
        "/map" => http::html(views::pages::map(lang, origin, checked().await?.as_deref()), 200, Policy::Map),
        "/funding" => {
            let sort = req.url()?.query_pairs().find(|(key, _)| key == "sort").map(|(_, v)| db::FundingSort::parse(&v)).unwrap_or_default();
            let (rows, totals) = db::funding_list(&db, sort).await?;
            http::html(views::funding::list(lang, origin, sort, &rows, &totals), 200, Policy::Page)
        }
        "/status" => http::html(views::pages::status(lang, origin, &db::status(&db).await?, worker::Date::now().as_millis() as i64), 200, Policy::Page),
        "/api/v1/status" => {
            let status = db::status(&db).await?;
            let (body, healthy) = api::status_json(&status, worker::Date::now().as_millis() as i64);
            let mut response = Response::from_bytes(body.into_bytes())?.with_status(if healthy { 200 } else { 503 });
            response.headers_mut().set("Content-Type", "application/json; charset=utf-8")?;
            response.headers_mut().set("Cache-Control", "no-store")?;
            response.headers_mut().set("Access-Control-Allow-Origin", "*")?;
            Ok(response)
        }
        "/liability" => {
            let url = req.url()?;
            let param = |name: &str| url.query_pairs().find(|(key, _)| key == name).map(|(_, v)| v.chars().take(80).collect::<String>()).unwrap_or_default();
            let (wing, contractor) = (param("wing"), param("c"));
            let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64);
            let data = db::liability(&db, &wing, &contractor, &today.to_iso(), &today.plus_days(views::liability::SOON_DAYS).to_iso()).await?;
            http::html(views::liability::render(lang, origin, &wing, &contractor, &data, today), 200, Policy::Page)
        }
        "/api/v1/liability" => http::data(api::liability_json(&db::liability_export(&db).await?), "application/json; charset=utf-8"),
        "/api/v1/liability.csv" => http::data(api::liability_csv(&db::liability_export(&db).await?), "text/csv; charset=utf-8"),
        "/api/v1/funding" => {
            let (rows, checked) = db::funding_export(&db).await?;
            http::data(api::funding_json(&rows, checked.as_deref()), "application/json; charset=utf-8")
        }
        "/api/v1/funding.csv" => http::data(api::funding_csv(&db::funding_export(&db).await?.0), "text/csv; charset=utf-8"),
        "/api/v1/projects" => {
            let rows = db::export(&db).await?;
            http::data(api::projects_json(&rows, db::last_checked(&db).await?.as_deref()), "application/json; charset=utf-8")
        }
        "/api/v1/projects.csv" => http::data(api::projects_csv(&db::export(&db).await?), "text/csv; charset=utf-8"),
        "/api/v1/projects.geojson" => http::data(api::projects_geojson(&db::export(&db).await?), "application/geo+json; charset=utf-8"),
        _ => {
            if let Some(code) = rest.strip_prefix("/p/").filter(|c| is_code(c)) {
                if let Some((project, data)) = load_project(&db, code).await? {
                    let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64);
                    return http::html(views::project::render(lang, origin, &project, &data, today), 200, Policy::Page);
                }
            } else if let Some(code) = rest.strip_prefix("/api/v1/projects/").filter(|c| is_code(c)) {
                return match load_project(&db, code).await? {
                    Some((project, data)) => http::data(api::project_json(&project, &data), "application/json; charset=utf-8"),
                    None => Response::error("{\"error\":\"no such project\"}", 404),
                };
            } else if let Some(reference) = rest.strip_prefix("/f/").filter(|r| is_code(r)) {
                if let Some(data) = db::funding_project(&db, reference).await? {
                    let project = serde_json::from_str(&data.row.record_json).map_err(|e| Error::RustError(e.to_string()))?;
                    return http::html(views::funding::detail(lang, origin, &project, &data), 200, Policy::Page);
                }
            } else if let Some(id) = rest.strip_prefix("/snapshot/").and_then(|id| id.parse::<i64>().ok()) {
                return snapshot(env, &db, id).await;
            }
            http::html(views::pages::not_found(lang, origin), 404, Policy::Page)
        }
    }
}

async fn load_project(db: &worker::d1::D1Database, code: &str) -> Result<Option<(Project, db::ProjectPage)>> {
    let Some(data) = db::project(db, code).await? else { return Ok(None) };
    let project = serde_json::from_str(&data.row.record_json).map_err(|e| Error::RustError(e.to_string()))?;
    Ok(Some((project, data)))
}

/// The stored copy of a source page, as a download. Never served as HTML from our origin.
async fn snapshot(env: &Env, db: &worker::d1::D1Database, id: i64) -> Result<Response> {
    let Some(row) = db::snapshot(db, id).await? else { return Response::error("No such snapshot", 404) };
    let Some(object) = env.bucket("BUCKET")?.get(row.r2_key.as_str()).execute().await? else {
        return Response::error("Snapshot file is missing", 404);
    };
    let Some(body) = object.body() else { return Response::error("Snapshot file is empty", 404) };
    let headers = Headers::new();
    headers.set("Content-Type", "text/plain; charset=utf-8")?;
    let filename: String = row.r2_key.rsplit('/').next().unwrap_or("snapshot").chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_')).collect();
    headers.set("Content-Disposition", &format!("attachment; filename=\"{filename}\""))?;
    headers.set("Cache-Control", "public, max-age=31536000, immutable")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("X-Robots-Tag", "noindex")?;
    Ok(Response::from_body(body.response_body()?)?.with_headers(headers))
}

/// Serves byte ranges of the basemap archive. The map library reads a few kilobytes at a time.
async fn tiles(req: &Request, env: &Env) -> Result<Response> {
    let bucket = env.bucket("BUCKET")?;
    if req.method() == Method::Head {
        return match bucket.head(TILES_KEY).await? {
            Some(_) => Ok(Response::empty()?.with_headers(Headers::from_iter([("Accept-Ranges", "bytes")]))),
            None => Response::error("Basemap not uploaded", 404),
        };
    }
    let range = req.headers().get("Range")?.and_then(|h| parse_range(&h));
    let get = bucket.get(TILES_KEY);
    let get = match range {
        Some((start, Some(end))) => get.range(worker::Range::OffsetWithLength { offset: start, length: end - start + 1 }),
        Some((start, None)) => get.range(worker::Range::OffsetToEnd { offset: start }),
        None => get,
    };
    let Some(object) = get.execute().await? else { return Response::error("Basemap not uploaded", 404) };
    let size = object.size();
    let etag = object.http_etag();
    let Some(body) = object.body() else { return Response::error("Basemap is empty", 404) };

    let headers = Headers::new();
    headers.set("Content-Type", "application/octet-stream")?;
    headers.set("Accept-Ranges", "bytes")?;
    headers.set("ETag", &etag)?;
    headers.set("Cache-Control", "public, max-age=86400")?;
    let response = Response::from_body(body.response_body()?)?;
    Ok(match range {
        Some((start, end)) => {
            let end = end.unwrap_or(size.saturating_sub(1)).min(size.saturating_sub(1));
            headers.set("Content-Range", &format!("bytes {start}-{end}/{size}"))?;
            response.with_status(206).with_headers(headers)
        }
        None => response.with_headers(headers),
    })
}

/// Parses `bytes=start-end` or `bytes=start-`. Other forms are served as the whole file.
fn parse_range(header: &str) -> Option<(u64, Option<u64>)> {
    let (start, end) = header.trim().strip_prefix("bytes=")?.split_once('-')?;
    let start = start.parse().ok()?;
    let end = if end.is_empty() { None } else { Some(end.parse::<u64>().ok().filter(|e| *e >= start)?) };
    Some((start, end))
}

fn filter_from(req: &Request) -> Result<db::Filter> {
    let mut filter = db::Filter { page: 1, ..db::Filter::default() };
    for (key, value) in req.url()?.query_pairs() {
        let value: String = value.trim().chars().take(120).collect();
        match key.as_ref() {
            "q" => filter.q = value,
            "dept" => filter.department = value,
            "lac" => filter.constituency = value,
            "stage" => filter.stage = value,
            "flag" => filter.flag = value,
            "sort" => filter.sort = db::Sort::parse(&value),
            "page" => filter.page = value.parse().unwrap_or(1).clamp(1, 10_000),
            _ => {}
        }
    }
    Ok(filter)
}

/// Project codes look like `PWD016-05-01`. Anything else is not worth a database query.
fn is_code(s: &str) -> bool {
    !s.is_empty() && s.len() <= 40 && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

async fn admin(mut req: Request, env: &Env) -> Result<Response> {
    if req.path() != "/admin/ingest" {
        return Response::error("Not found", 404);
    }
    if !authorised(&req, env) {
        return Response::error("Unauthorised", 401);
    }
    let url = req.url()?;
    let param = |name: &str| url.query_pairs().find(|(key, _)| key == name).map(|(_, value)| value.into_owned());
    let started = runs::now_iso();
    if param("source").as_deref() == Some("status") {
        // `details` caps how many work tables this run reads, so a first load can be done in steps.
        let details = param("details").and_then(|n| n.parse().ok()).unwrap_or(funding::DEFAULT_DETAILS);
        let result = funding::run(env, details).await;
        runs::record(env, 2, "manual", &started, &result).await;
        return Response::from_json(&result?);
    }
    if param("source").as_deref() == Some("liability") {
        let result = liability::run(env, true).await;
        runs::record(env, 3, "manual", &started, &result).await;
        return Response::from_json(&result?);
    }
    // A body is a copy of the dashboard page pushed from elsewhere; without one we fetch it.
    let body = req.bytes().await?;
    let result = ingest::run(env, (!body.is_empty()).then_some(body)).await;
    runs::record(env, 1, "manual", &started, &result).await;
    Response::from_json(&result?)
}

/// `Authorization: Bearer <INGEST_TOKEN>`, compared without short-circuiting.
fn authorised(req: &Request, env: &Env) -> bool {
    let Ok(secret) = env.secret("INGEST_TOKEN") else { return false };
    let expected = format!("Bearer {}", secret.to_string());
    let Ok(Some(given)) = req.headers().get("Authorization") else { return false };
    expected.len() == given.len() && expected.bytes().zip(given.bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}
