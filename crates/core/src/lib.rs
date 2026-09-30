//! Pure domain logic for Kanakku: source parsing, flag rules, formatting and UI strings.
//! No wasm or Cloudflare dependencies, so everything here is tested natively.

pub mod changes;
pub mod date;
pub mod flags;
pub mod fmt;
pub mod gaps;
pub mod i18n;
pub mod kiifb;
pub mod model;

pub use date::Date;
