//! Response helpers: security headers, cache headers and ETags.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use worker::{Headers, Response, Result};

/// The whole stylesheet ships inside every page, so a page is a single request.
pub const CSS: &str = include_str!("app.css");

/// Browsers keep a page for a minute; the edge keeps it for five and may serve it stale while refreshing.
const CACHE_CONTROL: &str = "public, max-age=60, s-maxage=300, stale-while-revalidate=86400";

#[derive(Clone, Copy, PartialEq)]
pub enum Policy {
    /// No scripts at all; only the inlined stylesheet.
    Page,
    /// The map page additionally runs Leaflet from our own origin.
    Map,
}

fn csp(policy: Policy) -> String {
    static STYLE_HASH: OnceLock<String> = OnceLock::new();
    let style = STYLE_HASH.get_or_init(|| base64(&Sha256::digest(CSS.as_bytes())));
    let base = "default-src 'none'; img-src 'self' data: blob:; form-action 'self'; base-uri 'none'; frame-ancestors 'none'";
    match policy {
        Policy::Page => format!("{base}; style-src 'sha256-{style}'"),
        Policy::Map => format!("{base}; style-src 'self' 'sha256-{style}'; script-src 'self'; connect-src 'self'"),
    }
}

pub fn html(body: String, status: u16, policy: Policy) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", "text/html; charset=utf-8")?;
    headers.set("Content-Security-Policy", &csp(policy))?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    if status == 200 {
        headers.set("Cache-Control", CACHE_CONTROL)?;
        headers.set("ETag", &etag(body.as_bytes()))?;
    } else {
        headers.set("Cache-Control", "no-store")?;
    }
    Ok(Response::from_bytes(body.into_bytes())?.with_status(status).with_headers(headers))
}

/// Open data: cacheable, and readable from any origin.
pub fn data(body: String, content_type: &str) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", content_type)?;
    headers.set("Cache-Control", CACHE_CONTROL)?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("ETag", &etag(body.as_bytes()))?;
    Ok(Response::from_bytes(body.into_bytes())?.with_headers(headers))
}

fn etag(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    format!("\"{}\"", base64(&digest[..12]))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Percent-encodes one path segment.
pub fn encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
