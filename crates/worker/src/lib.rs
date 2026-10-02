//! Kanakku on Cloudflare Workers: server-rendered pages, open data, and the nightly ingest.

use kanakku_core::i18n::Lang;
use kanakku_core::model::Project;
use kanakku_core::names;
use worker::{
    console_error, console_log, event, Cache, Context, Env, Error, Headers, Method, Request, Response, Result, ScheduleContext,
    ScheduledEvent,
};

mod api;
mod cards;
mod db;
mod district;
mod funding;
mod http;
mod icons;
mod ingest;
mod liability;
mod og;
mod runs;
mod views;

use http::Policy;

const TILES_PATH: &str = "/tiles/kerala.pmtiles";
const TILES_KEY: &str = "tiles/kerala.pmtiles";

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    if let Some(to) = canonical_redirect(&req, &env)? {
        return Response::redirect_with_status(to, 301);
    }
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

/// Where a page request on `www.` or the workers.dev address belongs: the same path and query on
/// `CANONICAL_HOST`. Other hosts (local development) and other methods are left alone, so the
/// admin endpoint keeps working on workers.dev.
fn canonical_redirect(req: &Request, env: &Env) -> Result<Option<worker::Url>> {
    let Ok(canonical) = env.var("CANONICAL_HOST").map(|v| v.to_string()) else { return Ok(None) };
    if canonical.is_empty() || !matches!(req.method(), Method::Get | Method::Head) {
        return Ok(None);
    }
    let url = req.url()?;
    let host = url.host_str().unwrap_or_default();
    if host == canonical || !(host.ends_with(".workers.dev") || host == format!("www.{canonical}")) {
        return Ok(None);
    }
    let mut to = url.clone();
    to.set_host(Some(&canonical)).map_err(|e| Error::RustError(e.to_string()))?;
    let _ = to.set_scheme("https");
    let _ = to.set_port(None);
    Ok(Some(to))
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
    let contact = env.var("CONTACT").map(|v| v.to_string()).unwrap_or_default();
    // A search that finds nothing is noted, so the words it missed can be taught to the search.
    let person = req.headers().get("User-Agent")?.is_some_and(|ua| !runs::is_robot(&ua));
    let missed = |surface: &'static str, q: String| async move {
        if person {
            runs::count_miss(env, surface, lang, &q).await;
        }
    };

    match rest {
        // The list used to live at the root; keep old search and filter links working.
        "/" if req.url()?.query().is_some_and(|q| !q.is_empty()) => {
            let mut to = req.url()?;
            to.set_path(&format!("{}/projects", lang.prefix()));
            Response::redirect_with_status(to, 301)
        }
        "/" => http::html(views::home::render(lang, origin, &db::home(&db, "").await?, ""), 200, Policy::Page),
        "/projects" => {
            let filter = filter_from(req)?;
            let listing = db::list(&db, &filter).await?;
            // Only a search on its own: with other filters set, an empty list says little about the words.
            if listing.totals.total == 0 && !filter.q.is_empty() && (db::Filter { q: String::new(), ..filter.clone() }).is_empty() {
                missed("projects", filter.q.clone()).await;
            }
            http::html(views::list::render(lang, origin, &filter, &listing), 200, Policy::Page)
        }
        "/methodology" => http::html(views::pages::methodology(lang, origin, checked().await?.as_deref()), 200, Policy::Page),
        "/about" => http::html(views::pages::about(lang, origin, &contact), 200, Policy::Page),
        "/data" => http::html(views::pages::data(lang, origin, checked().await?.as_deref()), 200, Policy::Page),
        "/map" => http::html(views::pages::map(lang, origin, checked().await?.as_deref()), 200, Policy::Map),
        "/funding" => {
            let url = req.url()?;
            let param = |name: &str| url.query_pairs().find(|(key, _)| key == name).map(|(_, v)| v.trim().chars().take(80).collect::<String>()).unwrap_or_default();
            let query = views::funding::Query {
                sort: db::FundingSort::parse(&param("sort")),
                district: names::canonical_district(&param("district")).unwrap_or("").to_string(),
                q: param("q"),
                flagged: !param("flag").is_empty(),
                page: param("page").parse().unwrap_or(1).clamp(1, 10_000),
            };
            let listing = db::funding_list(&db, query.sort, &query.district, &query.q, query.flagged, query.page).await?;
            if listing.matching == 0 && !query.q.is_empty() && query.district.is_empty() && !query.flagged {
                missed("funding", query.q.clone()).await;
            }
            http::html(views::funding::list(lang, origin, &query, &listing), 200, Policy::Page)
        }
        "/contractors" => {
            let url = req.url()?;
            let param = |name: &str| url.query_pairs().find(|(key, _)| key == name).map(|(_, v)| v.trim().chars().take(80).collect::<String>()).unwrap_or_default();
            let (q, page) = (param("q"), param("page").parse().unwrap_or(1).clamp(1, 10_000));
            let (rows, matching) = db::contractors(&db, &q, page).await?;
            if matching == 0 && !q.is_empty() {
                missed("contractors", q.clone()).await;
            }
            http::html(views::entities::contractors(lang, origin, &q, page, &rows, matching), 200, Policy::Page)
        }
        "/agencies" => http::html(views::entities::agencies(lang, origin, &db::agencies(&db).await?), 200, Policy::Page),
        "/offline" => http::html(views::pages::offline(lang, origin), 200, Policy::Page),
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
            let param = |name: &str| url.query_pairs().find(|(key, _)| key == name).map(|(_, v)| v.trim().chars().take(80).collect::<String>()).unwrap_or_default();
            let filter = db::LiabilityFilter {
                wing: param("wing"),
                contractor: param("c"),
                district: names::canonical_district(&param("district")).unwrap_or("").to_string(),
                page: param("page").parse().unwrap_or(1).clamp(1, 10_000),
            };
            let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64);
            let data = db::liability(&db, &filter, &today.to_iso(), &today.plus_days(views::liability::SOON_DAYS).to_iso()).await?;
            http::html(views::liability::render(lang, origin, &filter, &data, today), 200, Policy::Page)
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
        "/api/v1/changes" | "/api/v1/flags" | "/api/v1/snapshots" => api_list(req, &db, rest).await,
        "/api/v1/projects.geojson" => http::data(api::projects_geojson(&db::export(&db).await?), "application/geo+json; charset=utf-8"),
        _ => {
            if let Some(code) = rest.strip_prefix("/p/").filter(|c| is_code(c)) {
                if let Some((project, data)) = load_project(&db, code).await? {
                    let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64);
                    return http::html(views::project::render(lang, origin, &project, &data, today, &contact), 200, Policy::Page);
                }
            } else if let Some(code) = rest.strip_prefix("/api/v1/projects/").filter(|c| is_code(c)) {
                return match load_project(&db, code).await? {
                    Some((project, data)) => http::data(api::project_json(&project, &data), "application/json; charset=utf-8"),
                    None => Response::error("{\"error\":\"no such project\"}", 404),
                };
            } else if let Some(code) = path.strip_prefix("/og/p/").and_then(|c| c.strip_suffix(".png")).filter(|c| is_code(c)) {
                if let Some((project, data)) = load_project(&db, code).await? {
                    return share_image(&cards::project(&project, &data));
                }
            } else if let Some(reference) = path.strip_prefix("/og/f/").and_then(|r| r.strip_suffix(".png")).filter(|r| is_code(r)) {
                if let Some(data) = db::funding_project(&db, reference).await? {
                    let project = serde_json::from_str(&data.row.record_json).map_err(|e| Error::RustError(e.to_string()))?;
                    return share_image(&cards::funded(&project));
                }
            } else if let Some(district) = rest.strip_prefix("/d/").and_then(names::district_from_slug) {
                return http::html(views::home::render(lang, origin, &db::home(&db, district).await?, district), 200, Policy::Page);
            } else if let Some(key) = rest.strip_prefix("/c/").filter(|k| is_code(k)) {
                let data = db::contractor(&db, key).await?;
                if !data.works.is_empty() || !data.liability.is_empty() {
                    let today = kanakku_core::Date::from_unix_ms_ist(worker::Date::now().as_millis() as i64);
                    return http::html(views::entities::contractor(lang, origin, key, &data, today), 200, Policy::Page);
                }
            } else if let Some(key) = rest.strip_prefix("/a/").filter(|k| is_code(k)) {
                if let Some(data) = db::agency(&db, key).await? {
                    return http::html(views::entities::agency(lang, origin, &data), 200, Policy::Page);
                }
            } else if let Some(reference) = rest.strip_prefix("/f/").filter(|r| is_code(r)) {
                if let Some(data) = db::funding_project(&db, reference).await? {
                    let project = serde_json::from_str(&data.row.record_json).map_err(|e| Error::RustError(e.to_string()))?;
                    return http::html(views::funding::detail(lang, origin, &project, &data, &contact), 200, Policy::Page);
                }
            } else if let Some(id) = rest.strip_prefix("/snapshot/").and_then(|id| id.parse::<i64>().ok()) {
                return snapshot(env, &db, id).await;
            }
            http::html(views::pages::not_found(lang, origin), 404, Policy::Page)
        }
    }
}

/// The paged endpoints. Bad parameters get a 400 that says which one, rather than an empty list.
async fn api_list(req: &Request, db: &worker::d1::D1Database, path: &str) -> Result<Response> {
    let url = req.url()?;
    let param = |name: &str| url.query_pairs().find(|(key, _)| key == name).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
    let bad = |what: &str| -> Result<Response> {
        let mut response = Response::from_json(&serde_json::json!({ "error": what }))?.with_status(400);
        response.headers_mut().set("Access-Control-Allow-Origin", "*")?;
        Ok(response)
    };
    let page: u32 = match param("page").as_str() {
        "" => 1,
        p => match p.parse::<u32>() {
            Ok(n) if (1..=100_000).contains(&n) => n,
            _ => return bad("page must be a whole number from 1"),
        },
    };
    let district = match param("district").as_str() {
        "" => String::new(),
        "-" => "-".to_string(),
        d => match names::canonical_district(d) {
            Some(d) => d.to_string(),
            None => return bad("district must be one of Kerala's 14 districts, or - for none"),
        },
    };
    // The same address with the page moved on, when there is a next page.
    let next = |total: u32| -> Option<String> {
        (page * db::API_PAGE_SIZE < total).then(|| {
            let mut pairs: Vec<String> = url.query_pairs().filter(|(k, _)| k != "page").map(|(k, v)| views::pair(&k, &v)).collect();
            pairs.push(format!("page={}", page + 1));
            format!("{}?{}", url.path(), pairs.join("&"))
        })
    };
    let body = match path {
        "/api/v1/changes" => {
            let since = param("since");
            let valid = since.is_empty() || (since.len() == 10 && kanakku_core::Date::parse_iso(&since).is_some());
            if !valid {
                return bad("since must be a date written yyyy-mm-dd");
            }
            let (rows, total) = db::changes(db, &since, &district, page).await?;
            api::changes_json(&rows, page, total, next(total))
        }
        "/api/v1/flags" => {
            let status = match param("status").as_str() {
                "" | "open" => "open",
                "cleared" => "cleared",
                "all" => "all",
                _ => return bad("status must be open, cleared or all"),
            };
            let kind = match param("type").as_str() {
                "" => "",
                t => match kanakku_core::flags::FlagKind::parse(t) {
                    Some(kind) => kind.as_str(),
                    None => return bad("type is not a flag type; see /methodology#rules"),
                },
            };
            let (rows, total) = db::flags(db, status, kind, &district, page).await?;
            api::flags_json(&rows, page, total, next(total))
        }
        _ => {
            let source = match param("source").as_str() {
                "" => 0,
                s => match s.parse::<u32>() {
                    Ok(n) if (1..=3).contains(&n) => n,
                    _ => return bad("source must be 1 (dashboard), 2 (status page) or 3 (PWD liability list)"),
                },
            };
            let (rows, total) = db::snapshots(db, source, page).await?;
            api::snapshots_json(&rows, page, total, next(total))
        }
    };
    http::data(body, "application/json; charset=utf-8")
}

async fn load_project(db: &worker::d1::D1Database, code: &str) -> Result<Option<(Project, db::ProjectPage)>> {
    let Some(data) = db::project(db, code).await? else { return Ok(None) };
    let project = serde_json::from_str(&data.row.record_json).map_err(|e| Error::RustError(e.to_string()))?;
    Ok(Some((project, data)))
}

/// A share image as a response. Drawn once and then kept at the edge for a day.
fn share_image(card: &cards::Owned) -> Result<Response> {
    let Some(png) = og::render(&card.borrow()) else { return Response::error("Could not draw the image", 500) };
    let headers = Headers::new();
    headers.set("Content-Type", "image/png")?;
    headers.set("Cache-Control", "public, max-age=3600, s-maxage=86400")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    Ok(Response::from_bytes(png)?.with_headers(headers))
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
            "district" if value == "-" => filter.district = value,
            "district" => filter.district = names::canonical_district(&value).unwrap_or("").to_string(),
            "agency" => filter.agency = value,
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
    !s.is_empty() && s.len() <= 90 && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
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
