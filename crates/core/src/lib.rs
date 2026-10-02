//! Pure domain logic for Kanakku: source parsing, flag rules, formatting and UI strings.
//! No wasm or Cloudflare dependencies, so everything here is tested natively.

pub mod aspnet;
pub mod changes;
pub mod constituencies;
pub mod date;
pub mod entity;
pub mod flags;
pub mod fmt;
pub mod gaps;
pub mod i18n;
pub mod kiifb;
pub mod kiifb_status;
pub mod mail;
pub mod model;
pub mod names;
pub mod pwd_dlp;
pub mod search;
pub mod sitemap;
pub mod stage;
pub mod sulekha;
pub mod title;
pub mod usage;
pub mod visits;

pub use date::Date;
