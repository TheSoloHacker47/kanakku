//! Parses a trimmed copy of the real KIIFB dashboard page (captured 2026-09-30).

use kanakku_core::flags::{evaluate, FlagKind, History};
use kanakku_core::kiifb::parse;
use kanakku_core::model::Project;
use kanakku_core::Date;

const PAGE: &[u8] = include_bytes!("fixtures/gis_sample.html");

fn projects() -> Vec<Project> {
    parse(PAGE, "Ernakulam").unwrap().projects
}

fn find<'a>(projects: &'a [Project], code: &str) -> &'a Project {
    projects.iter().find(|p| p.code == code).unwrap_or_else(|| panic!("{code} missing"))
}

#[test]
fn counts_the_whole_page_but_keeps_one_district() {
    let parsed = parse(PAGE, "Ernakulam").unwrap();
    assert_eq!(parsed.markers_total, 11);
    assert_eq!(parsed.works_total, 10);
    assert!(parsed.projects.iter().all(|p| p.district == "Ernakulam"));
    assert!(parsed.projects.iter().all(|p| !p.code.is_empty() && !p.title.is_empty()));
    let codes: Vec<_> = parsed.projects.iter().map(|p| p.code.as_str()).collect();
    let mut sorted = codes.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(codes, sorted, "projects are sorted by code and unique");
    assert!(parse(PAGE, "Thrissur").unwrap().projects.len() >= 2);
    assert!(parse(PAGE, "Atlantis").unwrap().projects.is_empty());
}

#[test]
fn segments_cover_exactly_the_json_values() {
    let parsed = parse(PAGE, "Ernakulam").unwrap();
    let [markers, transport] = parsed.segments;
    assert_eq!(PAGE[markers.start], b'[');
    assert_eq!(PAGE[markers.end - 1], b']');
    assert_eq!(PAGE[markers.end], b';');
    assert_eq!(PAGE[transport.start], b'{');
    assert_eq!(PAGE[transport.end - 1], b'}');
}

#[test]
fn marker_amounts_are_rupees_despite_the_crore_label() {
    let all = projects();
    let p = find(&all, "CIN001-01-06");
    assert_eq!(p.title, "Boundary Stone Laying in Chilavannoor, Perandoor, Thevara and Market Canals");
    assert_eq!(p.estimated_amount, Some(5_665_100_000));
    assert_eq!(p.expenditure, None);
    assert_eq!(p.executing_agency.as_deref(), Some("KOCHI METRO RAIL LIMITED"));
    assert_eq!(p.constituencies.len(), 1);
    let c = &p.constituencies[0];
    assert_eq!(c.name, "Ernakulam");
    assert_eq!(c.name_ml.as_deref(), Some("എറണാകുളം (82)"));
    assert_eq!(c.mla_name.as_deref(), Some("Shri T J Vinod"));
    assert_eq!(c.mla_name_ml.as_deref(), Some("ശ്രീ ടി ജെ വിനോദ്"));
    assert_eq!(p.status.as_deref(), Some("WBS Base Zero Approved"));
    assert_eq!(p.sites.len(), 1);
    assert!(p.works.is_empty());
}

#[test]
fn a_project_listed_under_two_constituencies_is_one_project_with_one_pin() {
    let all = projects();
    let p = find(&all, "FSH003-07-06");
    assert_eq!(p.sites.len(), 1);
    let names: Vec<_> = p.constituencies.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Thripunithura", "Ernakulam"]);
    assert_eq!(all.iter().filter(|p| p.code == "FSH003-07-06").count(), 1);
}

#[test]
fn transport_amounts_are_crore_and_dates_are_day_first() {
    let all = projects();
    let p = find(&all, "PWD016-05-01");
    assert_eq!(p.title, "69 Bridges");
    assert_eq!(p.estimated_amount, None, "KIIFB lists this project only as works");
    assert!(p.sites.is_empty());
    assert_eq!(p.constituencies[0].name, "Kochi");
    assert_eq!(p.constituencies[0].mla_name, None);
    let w = p.works.iter().find(|w| w.road_name.as_deref() == Some("Flyover at Vyttila")).unwrap();
    assert_eq!(w.contractor.as_deref(), Some("SREEDHANYACONSTRUCTIONCOMPANY"));
    assert_eq!(w.spv.as_deref(), Some("KRFB"));
    assert_eq!(w.as_amount, None);
    assert_eq!(w.fs_amount, Some(863_400_000));
    assert_eq!(w.contract_amount, Some(749_684_376));
    assert_eq!(w.paid_amount, Some(747_496_995));
    assert_eq!(w.scheduled_start, Date::parse_iso("2017-11-17"));
    assert_eq!(w.scheduled_end, Date::parse_iso("2026-01-31"));
    assert_eq!(w.physical_pct, Some(100.0));
    assert!(w.is_completed());
    let (lat, lng) = (w.lat.unwrap(), w.lng.unwrap());
    assert!((9.5..10.5).contains(&lat) && (76.0..77.0).contains(&lng), "{lat},{lng}");
    assert_eq!(p.works_amount(), Some(p.works.iter().filter_map(|w| w.fs_amount).sum()));
}

#[test]
fn a_work_attaches_to_the_marker_project_with_the_same_code() {
    let all = projects();
    let p = find(&all, "PWD001-09-05");
    assert!(!p.sites.is_empty());
    assert!(p.estimated_amount.is_some());
    assert_eq!(p.works.len(), 1);
    assert_eq!(p.works[0].road_name.as_deref(), Some("Kumbalangi Keltron- Keltron ferry bridge"));
}

#[test]
fn four_works_were_overdue_on_the_capture_date() {
    let today = Date::parse_iso("2026-09-30").unwrap();
    let mut overdue = Vec::new();
    for p in projects() {
        for f in evaluate(&p, today, &History::default()) {
            assert_eq!(f.kind, FlagKind::Overdue, "{}: {:?}", p.code, f);
            overdue.push((p.code.clone(), f.work_ref.unwrap(), f.value["days_overdue"].as_i64().unwrap()));
        }
    }
    overdue.sort();
    assert_eq!(
        overdue,
        [
            ("CIN001-07-01".to_string(), "#1".to_string(), 435),
            ("PWD007-01-02".to_string(), "Perumbavoor Bypass".to_string(), 299),
            ("PWD015-05-04".to_string(), "Muvattupuzha town portion".to_string(), 335),
            ("WRD022-10-01".to_string(), "Regulator with Lock at Puthenkavu".to_string(), 280),
        ]
    );
}
