//! Kanakku on Cloudflare Workers: server-rendered pages, open data, and the nightly ingest.

use kanakku_core::i18n::Lang;
use kanakku_core::model::Project;
use worker::{
    console_error, console_log, event, Cache, Context, Env, Error, Headers, Method, Request, Response, Result, ScheduleContext,
    ScheduledEvent,
};

mod api;
mod db;
mod http;
mod ingest;
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
    match ingest::run(&env, None).await {
        Ok(report) => console_log!("ingest ok: {}", serde_json::to_string(&report).unwrap_or_default()),
        Err(e) => console_error!("ingest failed: {e}"),
    }
}

/// Serves a GET from the edge cache when it can, and fills the cache when it cannot.
async fn cached(req: Request, env: &Env, ctx: &Context) -> Result<Response> {
    let use_cache = env.var("EDGE_CACHE").map(|v| v.to_string() == "on").unwrap_or(false);
    let cache = Cache::default();
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

    match rest {
        "/" => {
            let filter = filter_from(req)?;
            let listing = db::list(&db, &filter).await?;
            http::html(views::list::render(lang, &filter, &listing), 200, Policy::Page)
        }
        "/methodology" => http::html(views::pages::methodology(lang, db::last_checked(&db).await?.as_deref()), 200, Policy::Page),
        "/data" => http::html(views::pages::data(lang, db::last_checked(&db).await?.as_deref()), 200, Policy::Page),
        "/map" => http::html(views::pages::map(lang, db::last_checked(&db).await?.as_deref()), 200, Policy::Map),
        "/api/v1/projects" => {
            let rows = db::export(&db).await?;
            http::data(api::projects_json(&rows, db::last_checked(&db).await?.as_deref()), "application/json; charset=utf-8")
        }
        "/api/v1/projects.csv" => http::data(api::projects_csv(&db::export(&db).await?), "text/csv; charset=utf-8"),
        "/api/v1/projects.geojson" => http::data(api::projects_geojson(&db::export(&db).await?), "application/geo+json; charset=utf-8"),
        _ => {
            if let Some(code) = rest.strip_prefix("/p/").filter(|c| is_code(c)) {
                if let Some((project, data)) = load_project(&db, code).await? {
                    return http::html(views::project::render(lang, &project, &data), 200, Policy::Page);
                }
            } else if let Some(code) = rest.strip_prefix("/api/v1/projects/").filter(|c| is_code(c)) {
                return match load_project(&db, code).await? {
                    Some((project, data)) => http::data(api::project_json(&project, &data), "application/json; charset=utf-8"),
                    None => Response::error("{\"error\":\"no such project\"}", 404),
                };
            } else if let Some(id) = rest.strip_prefix("/snapshot/").and_then(|id| id.parse::<i64>().ok()) {
                return snapshot(env, &db, id).await;
            }
            http::html(views::pages::not_found(lang), 404, Policy::Page)
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
    let Some(object) = env.bucket("BUCKET")?.get(row.r2_key).execute().await? else {
        return Response::error("Snapshot file is missing", 404);
    };
    let Some(body) = object.body() else { return Response::error("Snapshot file is empty", 404) };
    let headers = Headers::new();
    headers.set("Content-Type", "text/plain; charset=utf-8")?;
    headers.set("Content-Disposition", &format!("attachment; filename=\"kiifb-{}.html\"", &row.sha256[..row.sha256.len().min(16)]))?;
    headers.set("Cache-Control", "public, max-age=31536000, immutable")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("X-Robots-Tag", "noindex")?;
    Ok(Response::from_body(body.response_body()?)?.with_headers(headers))
}

/// Serves byte ranges of the basemap archive. The map library reads a few kilobytes at a time.
async fn tiles(req: &Request, env: &Env) -> Result<Response> {
    let bucket = env.bucket("BUCKET")?;
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
            "status" => filter.status = value,
            "flag" => filter.flag = value,
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
    // A body is a copy of the dashboard page pushed from elsewhere; without one we fetch it.
    let body = req.bytes().await?;
    let report = ingest::run(env, (!body.is_empty()).then_some(body)).await?;
    Response::from_json(&report)
}

/// `Authorization: Bearer <INGEST_TOKEN>`, compared without short-circuiting.
fn authorised(req: &Request, env: &Env) -> bool {
    let Ok(secret) = env.secret("INGEST_TOKEN") else { return false };
    let expected = format!("Bearer {}", secret.to_string());
    let Ok(Some(given)) = req.headers().get("Authorization") else { return false };
    expected.len() == given.len() && expected.bytes().zip(given.bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}
