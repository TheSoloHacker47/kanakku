//! Kanakku on Cloudflare Workers: server-rendered pages, open data, and the nightly ingest.

use worker::{console_error, console_log, event, Context, Env, Method, Request, Response, Result, ScheduleContext, ScheduledEvent};

mod ingest;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    match route(req, &env).await {
        Ok(response) => Ok(response),
        Err(e) => {
            console_error!("request failed: {e}");
            Response::error("Something went wrong on our side.", 500)
        }
    }
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    match ingest::run(&env, None).await {
        Ok(report) => console_log!("ingest ok: {}", serde_json::to_string(&report).unwrap_or_default()),
        Err(e) => console_error!("ingest failed: {e}"),
    }
}

async fn route(mut req: Request, env: &Env) -> Result<Response> {
    let path = req.path();
    match (req.method(), path.as_str()) {
        (Method::Post, "/admin/ingest") => {
            if !authorised(&req, env) {
                return Response::error("Unauthorised", 401);
            }
            // A body is a copy of the dashboard page pushed from elsewhere; without one we fetch it.
            let body = req.bytes().await?;
            let report = ingest::run(env, (!body.is_empty()).then_some(body)).await?;
            Response::from_json(&report)
        }
        _ => Response::error("Not found", 404),
    }
}

/// `Authorization: Bearer <INGEST_TOKEN>`, compared without short-circuiting.
fn authorised(req: &Request, env: &Env) -> bool {
    let Ok(secret) = env.secret("INGEST_TOKEN") else { return false };
    let expected = format!("Bearer {}", secret.to_string());
    let Ok(Some(given)) = req.headers().get("Authorization") else { return false };
    expected.len() == given.len() && expected.bytes().zip(given.bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}
