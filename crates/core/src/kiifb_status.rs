//! Parser for KIIFB's project status page (`https://www.kiifb.org/prjStatus.jsp`).
//!
//! The page is a public form. Posted with a district it lists that district's projects with the
//! amount KIIFB approved and the amount it has released. Posted again with one project selected
//! it adds a panel listing that project's works, each with its approved and paid amounts.
//!
//! A "project" here is what the map dashboard calls a sub-project; its works are the dashboard's
//! contract packages. The two sources share no identifier, so `link` joins them on the figures
//! they both state.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::changes::Change;

pub const SOURCE_URL: &str = "https://www.kiifb.org/prjStatus.jsp";

/// The form's action. The page is only ever read through this one public form.
pub const FORM_URL: &str = "https://www.kiifb.org/prjStatus.jsp?shw";

/// Bump when the parser reads the same page differently, so stored records are rebuilt.
pub const PARSER_VERSION: u32 = 1;

const DISTRICTS: [&str; 14] = [
    "Thiruvananthapuram",
    "Kollam",
    "Pathanamthitta",
    "Alappuzha",
    "Kottayam",
    "Idukki",
    "Ernakulam",
    "Thrissur",
    "Palakkad",
    "Malappuram",
    "Kozhikode",
    "Wayanad",
    "Kannur",
    "Kasaragod",
];

#[derive(Debug, PartialEq)]
pub enum ParseError {
    NoProjectTable,
    NoDetailPanel,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NoProjectTable => write!(f, "no project rows in the KIIFB status page"),
            ParseError::NoDetailPanel => write!(f, "no project detail panel in the KIIFB status page"),
        }
    }
}

impl std::error::Error for ParseError {}

/// One project as KIIFB's status page states it. Amounts are whole rupees.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundedProject {
    /// KIIFB's own id for the row. Opaque, and only meaningful on the status page.
    pub reference: String,
    pub name: String,
    pub department: Option<String>,
    pub spv: Option<String>,
    /// The budget announcement the project sits under, e.g. "137 Roads".
    pub main_project: Option<String>,
    /// `None` while the project is still under evaluation.
    pub approved: Option<i64>,
    pub released: Option<i64>,
    pub status: Option<String>,
    pub works: Vec<FundedWork>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundedWork {
    pub name: String,
    pub spv: Option<String>,
    pub approved: Option<i64>,
    pub paid: Option<i64>,
    pub status: Option<String>,
}

impl FundedProject {
    /// A name for each work that stays stable between snapshots. KIIFB repeats some names
    /// within a project, so repeats are numbered.
    pub fn work_references(&self) -> Vec<String> {
        let mut seen: HashMap<&str, u32> = HashMap::new();
        self.works
            .iter()
            .map(|w| {
                let n = seen.entry(w.name.as_str()).or_insert(0);
                *n += 1;
                if *n == 1 {
                    w.name.clone()
                } else {
                    format!("{} #{n}", w.name)
                }
            })
            .collect()
    }

    pub fn works_paid(&self) -> i64 {
        self.works.iter().filter_map(|w| w.paid).sum()
    }
}

impl FundedWork {
    /// More has been paid than was approved for this work, by over 1%.
    /// Smaller differences are paise and rounding, and say nothing.
    pub fn paid_exceeds_approved(&self) -> bool {
        matches!((self.approved, self.paid), (Some(approved), Some(paid)) if approved > 0 && (paid - approved) * 100 > approved)
    }
}

/// The work table of one project, from the page returned when that project is selected.
#[derive(Debug, PartialEq)]
pub struct Detail<'a> {
    pub main_project: Option<String>,
    pub works: Vec<FundedWork>,
    /// The panel's own markup, which is what we keep as evidence.
    pub panel: &'a str,
}

/// KIIFB's number for a district in the form's drop-down.
pub fn district_id(name: &str) -> Option<u8> {
    DISTRICTS.iter().position(|d| d.eq_ignore_ascii_case(name.trim())).map(|i| i as u8 + 1)
}

/// The form body for a district, optionally with one project selected.
/// Returns `None` for a reference that is not plain letters and digits.
pub fn form_body(district: u8, selected: Option<&str>) -> Option<String> {
    let selected = selected.unwrap_or("");
    if !selected.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(format!(
        "optDeptName=0&hidDid=0&optDistrict={district}&hidSelDist={district}&optConstituency=0&hidSelConstituency=0&optStatus=ALL&hidStatus=ALL&hidSelProject={selected}"
    ))
}

/// Reads the project rows of the list. Works are filled in separately from each detail page.
pub fn parse_list(page: &str) -> Result<Vec<FundedProject>, ParseError> {
    const ROW: &str = "onclick=\"selectProject('";
    let mut out = Vec::new();
    let mut rest = page;
    while let Some(at) = rest.find(ROW) {
        rest = &rest[at + ROW.len()..];
        let Some(quote) = rest.find('\'') else { break };
        let reference = &rest[..quote];
        let row = &rest[..rest.find("</tr>").unwrap_or(rest.len())];
        let cells = cells(row);
        if let [department, name, spv, approved, released, status, ..] = cells.as_slice() {
            if !reference.is_empty() && !name.is_empty() {
                out.push(FundedProject {
                    reference: reference.to_string(),
                    name: name.clone(),
                    department: some(department),
                    spv: some(spv),
                    main_project: None,
                    approved: amount(approved).filter(|a| *a > 0),
                    released: amount(released),
                    status: some(status),
                    works: Vec::new(),
                });
            }
        }
    }
    if out.is_empty() {
        return Err(ParseError::NoProjectTable);
    }
    Ok(out)
}

/// Reads the detail panel of the selected project.
pub fn parse_detail(page: &str) -> Result<Detail<'_>, ParseError> {
    let start = page.find("id=\"dvVwPrjInfo\"").ok_or(ParseError::NoDetailPanel)?;
    let tail = &page[start..];
    let end = tail.find("</table>").map(|i| i + "</table>".len()).ok_or(ParseError::NoDetailPanel)?;
    let panel = &tail[..end];

    let main_project = panel.find("Main Project Name:").and_then(|i| bold_after(&panel[i..]));

    // KIIFB does not close the work rows, so each one runs from its opening tag to the next.
    let table = &panel[panel.find("Work Information").ok_or(ParseError::NoDetailPanel)?..];
    let body = table.find("</th>").map(|_| &table[table.rfind("</th>").unwrap_or(0)..]).unwrap_or(table);
    let mut works = Vec::new();
    for chunk in body.split("<tr>").skip(1) {
        let cells = cells(chunk);
        if let [number, name, spv, approved, paid, status, ..] = cells.as_slice() {
            if number.parse::<u32>().is_ok() && !name.is_empty() {
                works.push(FundedWork {
                    name: name.clone(),
                    spv: some(spv),
                    approved: amount(approved).filter(|a| *a > 0),
                    paid: amount(paid),
                    status: some(status),
                });
            }
        }
    }
    Ok(Detail { main_project, works, panel })
}

/// What changed in a project's record between two snapshots.
pub fn diff(old: &FundedProject, new: &FundedProject) -> Vec<Change> {
    let mut out = Vec::new();
    let mut push = |field: String, a: Option<String>, b: Option<String>| {
        if a != b {
            out.push(Change { field, old: a, new: b });
        }
    };
    let text = |v: Option<i64>| v.map(|v| v.to_string());

    push("name".into(), Some(old.name.clone()), Some(new.name.clone()));
    push("department".into(), old.department.clone(), new.department.clone());
    push("spv".into(), old.spv.clone(), new.spv.clone());
    push("status".into(), old.status.clone(), new.status.clone());
    push("approved_amount".into(), text(old.approved), text(new.approved));
    push("released_amount".into(), text(old.released), text(new.released));

    let (old_refs, new_refs) = (old.work_references(), new.work_references());
    for (i, name) in new_refs.iter().enumerate() {
        let after = &new.works[i];
        let Some(j) = old_refs.iter().position(|r| r == name) else {
            push(format!("work[{name}]"), None, Some("listed".into()));
            continue;
        };
        let before = &old.works[j];
        push(format!("work[{name}].status"), before.status.clone(), after.status.clone());
        push(format!("work[{name}].approved_amount"), text(before.approved), text(after.approved));
        push(format!("work[{name}].paid_amount"), text(before.paid), text(after.paid));
    }
    for name in old_refs.iter().filter(|r| !new_refs.contains(r)) {
        push(format!("work[{name}]"), Some("listed".into()), None);
    }
    out
}

/// A sub-project on the map dashboard, with the contract packages filed under it.
#[derive(Clone, Debug, Default)]
pub struct MapGroup {
    /// The sub-project code, or the project code where KIIFB gives no sub-project.
    pub key: String,
    pub estimate: i64,
    pub department: Option<String>,
    pub agency: Option<String>,
    /// Code and title of each package.
    pub packages: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Basis {
    /// Approved amount, department and implementing agency all agree, and no other record shares them.
    Figures,
    /// Approved amount and department agree, and the names share most of their words.
    FiguresAndName,
}

impl Basis {
    pub fn as_str(self) -> &'static str {
        match self {
            Basis::Figures => "figures",
            Basis::FiguresAndName => "figures+name",
        }
    }

    pub fn parse(s: &str) -> Option<Basis> {
        [Basis::Figures, Basis::FiguresAndName].into_iter().find(|b| b.as_str() == s)
    }
}

/// Joins status-page projects to map sub-projects. Returns, for each joined project reference,
/// the group key and what the join rests on. A project that could belong to more than one
/// group is left unjoined rather than guessed.
pub fn link(funded: &[FundedProject], groups: &[MapGroup]) -> HashMap<String, (String, Basis)> {
    let same_figures = |f: &FundedProject, g: &MapGroup| {
        f.approved == Some(g.estimate) && g.estimate > 0 && same_name(f.department.as_deref(), g.department.as_deref())
    };
    let mut out = HashMap::new();

    for f in funded {
        let candidates: Vec<&MapGroup> = groups.iter().filter(|g| same_figures(f, g)).collect();
        let strict: Vec<&MapGroup> =
            candidates.iter().copied().filter(|g| same_name(f.spv.as_deref(), g.agency.as_deref())).collect();
        if let [group] = strict.as_slice() {
            // The group must not fit another status project equally well.
            let rivals = funded.iter().filter(|o| same_figures(o, group) && same_name(o.spv.as_deref(), group.agency.as_deref())).count();
            if rivals == 1 {
                out.insert(f.reference.clone(), (group.key.clone(), Basis::Figures));
                continue;
            }
        }
        let mut scored: Vec<(f64, &MapGroup)> = candidates
            .iter()
            .map(|g| (g.packages.iter().map(|(_, title)| overlap(&f.name, title)).fold(0.0, f64::max), *g))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        match scored.as_slice() {
            [(best, group)] if *best >= 0.5 => {
                out.insert(f.reference.clone(), (group.key.clone(), Basis::FiguresAndName));
            }
            [(best, group), (next, _), ..] if *best >= 0.5 && best - next >= 0.2 => {
                out.insert(f.reference.clone(), (group.key.clone(), Basis::FiguresAndName));
            }
            _ => {}
        }
    }
    out
}

/// For each work of a joined project, the package whose published title is the same.
pub fn link_works(project: &FundedProject, group: &MapGroup) -> Vec<Option<String>> {
    project
        .works
        .iter()
        .map(|w| {
            let name = squash(&w.name);
            group.packages.iter().find(|(_, title)| !name.is_empty() && squash(title) == name).map(|(code, _)| code.clone())
        })
        .collect()
}

/// Letters and digits only, lower case: "Health & Family Welfare" and "HEALTH AND FAMILY WELFARE" compare equal.
fn squash(s: &str) -> String {
    s.replace('&', " and ").chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

fn same_name(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            let (a, b) = (squash(a), squash(b));
            !a.is_empty() && a == b
        }
        _ => false,
    }
}

/// Share of the shorter name's distinctive words that the other name also has.
fn overlap(a: &str, b: &str) -> f64 {
    const COMMON: [&str; 16] = [
        "kiifb", "kila", "the", "and", "for", "district", "ernakulam", "construction", "improvement", "improvements", "works", "work",
        "phase", "package", "govt", "government",
    ];
    let words = |s: &str| -> Vec<String> {
        let mut words: Vec<String> = s
            .split(|c: char| !c.is_ascii_alphanumeric())
            .map(|w| w.to_ascii_lowercase())
            .filter(|w| w.len() > 2 && !COMMON.contains(&w.as_str()))
            .collect();
        words.sort();
        words.dedup();
        words
    };
    let (a, b) = (words(a), words(b));
    let shorter = a.len().min(b.len());
    if shorter == 0 {
        return 0.0;
    }
    a.iter().filter(|w| b.contains(w)).count() as f64 / shorter as f64
}

/// The text of each `<td>` in a row.
fn cells(row: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(open) = rest.find("<td") {
        let Some(close) = rest[open..].find('>') else { break };
        let inner = &rest[open + close + 1..];
        let end = inner.find("</td>").unwrap_or(inner.len());
        out.push(text(&inner[..end]));
        rest = &inner[end..];
    }
    out
}

/// The first `<b>…</b>` after the start of `s`.
fn bold_after(s: &str) -> Option<String> {
    let inner = &s[s.find("<b>")? + 3..];
    some(&text(&inner[..inner.find("</b>")?]))
}

/// Visible text of a fragment: tags dropped, entities decoded, whitespace collapsed.
fn text(fragment: &str) -> String {
    let mut plain = String::with_capacity(fragment.len());
    let mut in_tag = false;
    for c in fragment.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                plain.push(' ');
            }
            _ if !in_tag => plain.push(c),
            _ => {}
        }
    }
    let plain = plain
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn some(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// "727385383.10" becomes 727385383. KIIFB states rupees, sometimes with paise.
fn amount(s: &str) -> Option<i64> {
    let value: f64 = s.replace(',', "").parse().ok()?;
    (value.is_finite() && value >= 0.0).then(|| value.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = include_str!("../tests/fixtures/kiifb_status_ernakulam.html");
    const DETAIL: &str = include_str!("../tests/fixtures/kiifb_status_detail.html");

    #[test]
    fn reads_every_project_row() {
        let projects = parse_list(LIST).unwrap();
        assert_eq!(projects.len(), 140);

        let road = projects.iter().find(|p| p.reference == "f4ue5vh2sj0q").unwrap();
        assert_eq!(road.name, "Improvements to Mamala - Piravom Road in Ernakulam District");
        assert_eq!(road.department.as_deref(), Some("Public Works Department"));
        assert_eq!(road.spv.as_deref(), Some("KERALA ROAD FUND BOARD"));
        assert_eq!(road.approved, Some(118_100_000));
        assert_eq!(road.released, Some(101_711_086));
        assert_eq!(road.status.as_deref(), Some("Project Approved"));
        assert!(road.works.is_empty());
    }

    #[test]
    fn rounds_paise_and_treats_zero_approval_as_not_yet_approved() {
        let projects = parse_list(LIST).unwrap();
        let cinema = projects.iter().find(|p| p.reference == "b8ye5vi1r").unwrap();
        assert_eq!(cinema.approved, Some(223_370_196));
        assert_eq!(cinema.released, Some(0));

        let pending = projects.iter().find(|p| p.reference == "d6wa9zf4u").unwrap();
        assert_eq!(pending.status.as_deref(), Some("Project Under Evaluation"));
        assert_eq!(pending.approved, None);
    }

    #[test]
    fn decodes_ampersands_in_names() {
        let projects = parse_list(LIST).unwrap();
        assert!(projects.iter().any(|p| p.department.as_deref() == Some("Health & Family Welfare")));
        assert!(projects.iter().all(|p| !p.name.contains("&amp;") && !p.name.contains('<')));
    }

    #[test]
    fn reads_the_unclosed_work_rows() {
        let detail = parse_detail(DETAIL).unwrap();
        assert_eq!(detail.main_project.as_deref(), Some("137 Roads"));
        assert_eq!(
            detail.works,
            [
                FundedWork {
                    name: "Mamala Piravom Road - Contract work".into(),
                    spv: Some("KERALA ROAD FUND BOARD (KRFB)".into()),
                    approved: Some(101_594_061),
                    paid: Some(99_753_086),
                    status: Some("Work Awarded".into()),
                },
                FundedWork {
                    name: "Improvements to Mamala-Piravom Road in Ernakulam District - Shifting utilities by KSEB".into(),
                    spv: Some("KERALA ROAD FUND BOARD (KRFB)".into()),
                    approved: Some(1_977_000),
                    paid: Some(1_958_000),
                    status: Some("Work Awarded".into()),
                },
                FundedWork {
                    name: "Improvements to Mamala-Piravom Road in Ernakulam District - KWA".into(),
                    spv: Some("KERALA ROAD FUND BOARD (KRFB)".into()),
                    approved: Some(11_100_000),
                    paid: Some(0),
                    status: Some("Work Started".into()),
                },
            ]
        );
        assert!(detail.panel.starts_with("id=\"dvVwPrjInfo\""));
        assert!(detail.panel.ends_with("</table>"));
        assert!(detail.panel.len() < 5_000);
    }

    #[test]
    fn a_list_page_has_no_detail_and_a_blank_page_has_no_list() {
        assert_eq!(parse_detail(LIST).unwrap_err(), ParseError::NoDetailPanel);
        assert_eq!(parse_list("<html></html>").unwrap_err(), ParseError::NoProjectTable);
    }

    #[test]
    fn form_body_selects_a_district_and_refuses_odd_references() {
        assert_eq!(district_id("Ernakulam"), Some(7));
        assert_eq!(district_id("ernakulam "), Some(7));
        assert_eq!(district_id("Mahe"), None);
        let body = form_body(7, Some("f4ue5vh2sj0q")).unwrap();
        assert!(body.contains("optDistrict=7&hidSelDist=7"));
        assert!(body.ends_with("hidSelProject=f4ue5vh2sj0q"));
        assert!(form_body(7, None).unwrap().ends_with("hidSelProject="));
        assert_eq!(form_body(7, Some("a&b=1")), None);
    }

    fn funded(reference: &str, name: &str, approved: i64, department: &str, spv: &str) -> FundedProject {
        FundedProject {
            reference: reference.into(),
            name: name.into(),
            approved: Some(approved),
            department: Some(department.into()),
            spv: Some(spv.into()),
            ..FundedProject::default()
        }
    }

    fn group(key: &str, estimate: i64, department: &str, agency: &str, titles: &[&str]) -> MapGroup {
        MapGroup {
            key: key.into(),
            estimate,
            department: Some(department.into()),
            agency: Some(agency.into()),
            packages: titles.iter().enumerate().map(|(i, t)| (format!("{key}-{:02}", i + 1), t.to_string())).collect(),
        }
    }

    #[test]
    fn joins_on_amount_department_and_agency() {
        let f = [funded("a1", "Ernakulam-Idukki Phase-3 Cluster", 118_100_000, "Public Works Department", "KERALA ROAD FUND BOARD")];
        let g = [
            group("PWD004-120", 118_100_000, "PUBLIC WORKS DEPARTMENT", "Kerala Road Fund Board", &["Mamala Piravom Road - Contract work"]),
            group("PWD004-121", 118_100_001, "Public Works Department", "KERALA ROAD FUND BOARD", &["Another road"]),
        ];
        assert_eq!(link(&f, &g).get("a1"), Some(&("PWD004-120".to_string(), Basis::Figures)));
    }

    #[test]
    fn a_shared_amount_in_another_department_is_not_a_join() {
        let f = [funded("a1", "Cluster phase 1", 120_000_000, "Water Resources Department", "KERALA WATER AUTHORITY")];
        let g = [group("GED008-06", 120_000_000, "General Education Department", "KITE", &["GHSS Elamakkara"])];
        assert!(link(&f, &g).is_empty());
    }

    #[test]
    fn two_equally_good_candidates_are_left_unjoined() {
        let f = [funded("a1", "School cluster", 50_000_000, "General Education Department", "KITE")];
        let g = [
            group("GED001-01", 50_000_000, "General Education Department", "KITE", &["GHSS North"]),
            group("GED001-02", 50_000_000, "General Education Department", "KITE", &["GHSS South"]),
        ];
        assert!(link(&f, &g).is_empty());

        // And the other way round: one group, two status projects that fit it.
        let f = [
            funded("a1", "School cluster A", 50_000_000, "General Education Department", "KITE"),
            funded("a2", "School cluster B", 50_000_000, "General Education Department", "KITE"),
        ];
        assert!(link(&f, &g[..1]).is_empty());
    }

    #[test]
    fn a_different_agency_needs_the_names_to_agree() {
        let g = [group("HED010-02", 90_000_000, "Higher Education Department", "KITCO LTD", &["Girls Hostel at Maharajas College Ernakulam"])];
        let named = [funded("a1", "Maharajas College Girls Hostel", 90_000_000, "Higher Education Department", "INKEL LIMITED")];
        assert_eq!(link(&named, &g).get("a1"), Some(&("HED010-02".to_string(), Basis::FiguresAndName)));

        let unrelated = [funded("a1", "Indoor stadium at Kalamassery", 90_000_000, "Higher Education Department", "INKEL LIMITED")];
        assert!(link(&unrelated, &g).is_empty());
    }

    #[test]
    fn works_join_to_packages_with_the_same_title() {
        let mut f = funded("a1", "Mamala road", 118_100_000, "Public Works Department", "KRFB");
        f.works = vec![
            FundedWork { name: "Mamala Piravom Road - Contract work".into(), ..FundedWork::default() },
            FundedWork { name: "Shifting utilities by KSEB".into(), ..FundedWork::default() },
        ];
        let g = group("PWD004-120", 118_100_000, "Public Works Department", "KRFB", &["MAMALA PIRAVOM ROAD – Contract Work"]);
        assert_eq!(link_works(&f, &g), [Some("PWD004-120-01".to_string()), None]);
    }

    #[test]
    fn diff_reports_money_and_work_changes() {
        let mut old = funded("a1", "Road", 100, "PWD", "KRFB");
        old.released = Some(40);
        old.works = vec![
            FundedWork { name: "Package".into(), paid: Some(10), ..FundedWork::default() },
            FundedWork { name: "Package".into(), paid: Some(5), ..FundedWork::default() },
        ];
        assert!(diff(&old, &old).is_empty());

        let mut new = old.clone();
        new.released = Some(55);
        new.works[1].paid = Some(20);
        new.works.push(FundedWork { name: "Utility shifting".into(), ..FundedWork::default() });
        let changes: Vec<_> = diff(&old, &new).into_iter().map(|c| (c.field, c.old, c.new)).collect();
        assert_eq!(
            changes,
            [
                ("released_amount".to_string(), Some("40".to_string()), Some("55".to_string())),
                ("work[Package #2].paid_amount".to_string(), Some("5".to_string()), Some("20".to_string())),
                ("work[Utility shifting]".to_string(), None, Some("listed".to_string())),
            ]
        );
    }

    #[test]
    fn flags_a_work_paid_beyond_its_approval() {
        let over = FundedWork { approved: Some(1_000), paid: Some(1_011), ..FundedWork::default() };
        let within = FundedWork { approved: Some(1_000), paid: Some(1_000), ..FundedWork::default() };
        // One rupee over ₹87 lakh is rounding, not a finding.
        let rounding = FundedWork { approved: Some(8_738_240), paid: Some(8_738_241), ..FundedWork::default() };
        let exactly_one_percent = FundedWork { approved: Some(1_000), paid: Some(1_010), ..FundedWork::default() };
        assert!(!rounding.paid_exceeds_approved());
        assert!(!exactly_one_percent.paid_exceeds_approved());
        let unapproved = FundedWork { approved: None, paid: Some(1_000), ..FundedWork::default() };
        assert!(over.paid_exceeds_approved());
        assert!(!within.paid_exceeds_approved());
        assert!(!unapproved.paid_exceeds_approved());
    }
}
