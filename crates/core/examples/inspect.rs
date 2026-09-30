//! Parses a saved copy of the KIIFB dashboard and prints what Kanakku would ingest.
//! Usage: cargo run -p kanakku-core --release --example inspect -- page.html [district] [yyyy-mm-dd]

use kanakku_core::flags::{evaluate, History};
use kanakku_core::{kiifb, Date};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("path to a saved gis.kiifb.org page");
    let district = args.next().unwrap_or_else(|| "Ernakulam".into());
    let today = args.next().and_then(|d| Date::parse_iso(&d)).expect("today as yyyy-mm-dd");

    let page = std::fs::read(path).expect("readable file");
    let started = std::time::Instant::now();
    let parsed = kiifb::parse(&page, &district).expect("parse");
    let elapsed = started.elapsed();

    let works: usize = parsed.projects.iter().map(|p| p.works.len()).sum();
    let sites: usize = parsed.projects.iter().map(|p| p.sites.len()).sum();
    println!("page: {} pins, {} works ({} bytes, parsed in {elapsed:?})", parsed.markers_total, parsed.works_total, page.len());
    println!("{district}: {} projects, {sites} pins, {works} works", parsed.projects.len());
    for p in &parsed.projects {
        for f in evaluate(p, today, &History::default()) {
            println!("  {} {} {:?} {}", p.code, f.kind.as_str(), f.work_ref, f.value);
        }
    }
}
