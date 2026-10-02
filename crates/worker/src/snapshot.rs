//! Stored copies: every source's evidence goes to R2 and gets a row in `snapshots`, before any
//! record is derived from it.

use serde::Deserialize;
use worker::d1::D1Database;
use worker::{query, Env, Error, HttpMetadata, Result};

pub const HTML: &str = "text/html; charset=utf-8";
pub const TSV: &str = "text/tab-separated-values; charset=utf-8";

#[derive(Deserialize)]
pub struct Latest {
    pub id: i64,
    pub sha256: String,
}

/// The newest stored copy for a source, or for one address of it when `url` is given.
pub async fn latest(db: &D1Database, source_id: u32, url: Option<&str>) -> Result<Option<Latest>> {
    match url {
        None => query!(db, "SELECT id, sha256 FROM snapshots WHERE source_id = ?1 ORDER BY id DESC LIMIT 1", source_id)?,
        Some(url) => query!(db, "SELECT id, sha256 FROM snapshots WHERE source_id = ?1 AND url = ?2 ORDER BY id DESC LIMIT 1", source_id, url)?,
    }
    .first::<Latest>(None)
    .await
}

/// What to keep, and where.
pub struct Copy<'a> {
    pub source_id: u32,
    /// The address the copy came from (or describes, for an extract).
    pub url: &'a str,
    /// The R2 key.
    pub key: &'a str,
    pub content_type: &'a str,
    pub sha256: &'a str,
    pub now_iso: &'a str,
}

/// Puts the body in R2 and records it. Returns the snapshot id.
pub async fn store(env: &Env, db: &D1Database, copy: Copy<'_>, body: Vec<u8>) -> Result<i64> {
    let bytes = body.len();
    env.bucket("BUCKET")?
        .put(copy.key, body)
        .http_metadata(HttpMetadata { content_type: Some(copy.content_type.into()), ..HttpMetadata::default() })
        .execute()
        .await?;
    query!(
        db,
        "INSERT INTO snapshots (source_id, url, r2_key, sha256, bytes, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id, sha256",
        copy.source_id,
        copy.url,
        copy.key,
        copy.sha256,
        bytes,
        copy.now_iso,
    )?
    .first::<Latest>(None)
    .await?
    .map(|row| row.id)
    .ok_or_else(|| Error::RustError("snapshot insert returned no row".into()))
}
