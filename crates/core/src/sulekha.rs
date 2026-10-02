//! Sulekha, the local bodies' plan system: its public view of every plan project.
//!
//! The public view (see `docs/sulekha-spike.md`) is an ASP.NET form walked one choice at a time:
//! year → kind of local body → district (`gvState`) → local body (`gvStat`) → project list
//! (`gvProjects`, 20 to a page). Reading it is dormant until IKM agrees; this module holds the
//! parts that do not touch the network: reading each table, and where a slow, resumable read
//! across several nights has got to.

use crate::aspnet::{self, Grid};
use serde::{Deserialize, Serialize};

pub const SOURCE_ID: u32 = 4;
pub const URL: &str = "https://plan.lsgkerala.gov.in/formulation/Public.aspx";
pub const PARSER_VERSION: u32 = 1;

/// The year drop-down's value for 2025-26 (the spike found 29; older years count down).
pub const DEFAULT_YEAR: &str = "29";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    DistrictPanchayat,
    BlockPanchayat,
    Municipality,
    Corporation,
    GramaPanchayat,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::DistrictPanchayat, Kind::BlockPanchayat, Kind::Municipality, Kind::Corporation, Kind::GramaPanchayat];

    /// The value of the form's `drpType` drop-down.
    pub fn form_value(self) -> &'static str {
        match self {
            Kind::DistrictPanchayat => "1",
            Kind::BlockPanchayat => "2",
            Kind::Municipality => "3",
            Kind::Corporation => "4",
            Kind::GramaPanchayat => "5",
        }
    }

    /// Short key for configuration and storage: `dp`, `bp`, `m`, `c`, `gp`.
    pub fn key(self) -> &'static str {
        match self {
            Kind::DistrictPanchayat => "dp",
            Kind::BlockPanchayat => "bp",
            Kind::Municipality => "m",
            Kind::Corporation => "c",
            Kind::GramaPanchayat => "gp",
        }
    }

    pub fn parse(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key().eq_ignore_ascii_case(key.trim()))
    }
}

/// What to read, from the `SULEKHA` setting: `Ernakulam:gp,Ernakulam:m`. Empty means off.
pub fn targets(setting: &str) -> Vec<(String, Kind)> {
    setting
        .split(',')
        .filter_map(|item| {
            let (district, kind) = item.split_once(':')?;
            let district = crate::names::canonical_district(district.trim())?;
            Some((district.to_string(), Kind::parse(kind)?))
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum Error {
    /// The page lacks the table the walk expected: the site changed, or the session expired.
    MissingTable(&'static str),
    /// The table is there but a column we rely on is not.
    MissingColumn(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingTable(t) => write!(f, "Sulekha page has no {t} table"),
            Error::MissingColumn(c) => write!(f, "Sulekha table has no {c} column"),
        }
    }
}

/// One row of the district table (`gvState`) or the local-body table (`gvStat`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub name: String,
    pub projects: Option<u32>,
    /// Planned outlay, in rupees (the page prints lakh).
    pub planned: Option<i64>,
    /// The postback argument that opens this row, such as `Select$6`.
    pub select: String,
}

/// Rows of a summary table. The footer (state or district total) has no link and is skipped.
pub fn summaries(html: &str, grid_id: &'static str) -> Result<Vec<Summary>, Error> {
    let grid = aspnet::grid(html, grid_id).ok_or(Error::MissingTable(grid_id))?;
    let name = grid.column(&["DISTRICT", "LOCAL", "PANCHAYAT", "MUNICIPAL", "CORPORATION", "NAME"]).unwrap_or(1);
    let projects = grid.column(&["PROJECT"]);
    let total = grid.column(&["TOTAL"]);
    Ok(grid
        .rows
        .iter()
        .filter_map(|row| {
            let (_, select) = row.postbacks.iter().find(|(target, arg)| target == grid_id && arg.starts_with("Select$"))?;
            Some(Summary {
                name: row.cells.get(name)?.clone(),
                projects: projects.and_then(|c| row.cells.get(c)).and_then(|s| aspnet::number(s)).map(|n| n as u32),
                planned: total.and_then(|c| row.cells.get(c)).and_then(|s| aspnet::number(s)).map(|lakh| (lakh * 100_000.0).round() as i64),
                select: select.clone(),
            })
        })
        .collect())
}

/// One project from a local body's list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanProject {
    /// The number the list prints for it.
    pub number: String,
    /// The name as Sulekha publishes it, usually Malayalam.
    pub name: String,
    /// "Formulation": the amount planned, in rupees.
    pub planned: Option<i64>,
    /// "Expense": the amount spent so far, in rupees.
    pub spent: Option<i64>,
}

/// A page of the project list, and the page numbers its pager offers.
pub fn projects(html: &str) -> Result<(Vec<PlanProject>, Vec<u32>), Error> {
    let grid = aspnet::grid(html, "gvProjects").ok_or(Error::MissingTable("gvProjects"))?;
    project_rows(&grid)
}

fn project_rows(grid: &Grid) -> Result<(Vec<PlanProject>, Vec<u32>), Error> {
    let planned = grid.column(&["FORMULATION"]).ok_or(Error::MissingColumn("Formulation"))?;
    let spent = grid.column(&["EXPENSE", "EXPENDITURE"]).ok_or(Error::MissingColumn("Expense"))?;
    let name = grid.column(&["NAME"]).or(grid.column(&["PROJECT"])).ok_or(Error::MissingColumn("project name"))?;
    let number = grid.column(&["SL", "NO"]).filter(|&c| c != name).unwrap_or(0);
    let money = |s: Option<&String>| s.and_then(|s| aspnet::number(s)).map(|n| n.round() as i64);
    let rows = grid
        .rows
        .iter()
        .filter_map(|row| {
            let name = row.cells.get(name)?.trim();
            let number = row.cells.get(number)?.trim();
            // Footers and blank rows have no number.
            if name.is_empty() || number.is_empty() || !number.chars().any(|c| c.is_ascii_digit()) {
                return None;
            }
            Some(PlanProject { number: number.to_string(), name: name.to_string(), planned: money(row.cells.get(planned)), spent: money(row.cells.get(spent)) })
        })
        .collect();
    Ok((rows, grid.pages.clone()))
}

/// The evidence kept for one local body: our extract of its list, as tab-separated text.
pub fn extract(year: &str, district: &str, kind: Kind, local_body: &str, projects: &[PlanProject]) -> String {
    let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
    let mut out = format!(
        "# Sulekha public plan view, {URL}\n# year {year}, {district}, {}, {}\nnumber\tname\tplanned\tspent\n",
        kind.key(),
        clean(local_body)
    );
    for p in projects {
        let amount = |a: Option<i64>| a.map(|a| a.to_string()).unwrap_or_default();
        out.push_str(&format!("{}\t{}\t{}\t{}\n", clean(&p.number), clean(&p.name), amount(p.planned), amount(p.spent)));
    }
    out
}

/// Where a read spread over several nights has got to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    /// Index into the configured targets.
    pub target: usize,
    /// Index of the next local body within the target's district.
    pub local_body: usize,
    /// When the current pass started (UTC ISO); empty before the first.
    pub pass_started: String,
}

impl Cursor {
    /// After a local body is read: on to the next, or to the next target when that was the last.
    pub fn advance(&mut self, local_bodies: usize) {
        self.local_body += 1;
        if self.local_body >= local_bodies {
            self.target += 1;
            self.local_body = 0;
        }
    }

    pub fn finished(&self, targets: usize) -> bool {
        self.target >= targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISTRICT_PANCHAYATS: &str = include_str!("../tests/fixtures/sulekha-district-panchayats.html");

    #[test]
    fn reads_the_district_summary() {
        let rows = summaries(DISTRICT_PANCHAYATS, "gvState").unwrap();
        assert_eq!(rows.len(), 14);
        let ernakulam = rows.iter().find(|r| r.name == "Ernakulam").unwrap();
        assert_eq!(ernakulam.projects, Some(941));
        assert_eq!(ernakulam.planned, Some(1_298_991_000));
        assert_eq!(ernakulam.select, "Select$6");
        assert_eq!(summaries(DISTRICT_PANCHAYATS, "gvStat"), Err(Error::MissingTable("gvStat")));
    }

    fn list_page(rows: &str, pager: &str) -> String {
        format!(
            r#"<table id="gvProjects"><tr><th>Sl No</th><th>Project Name</th><th>Formulation</th><th>Expense</th><th></th></tr>{rows}{pager}</table>"#
        )
    }

    #[test]
    fn reads_a_project_list_page() {
        let html = list_page(
            r#"<tr><td>1</td><td>പദ്ധതി നിർവഹണ മോണിട്ടറിംഗ് ചെലവുകൾ</td><td>500000</td><td>321848</td><td><a href="javascript:__doPostBack('gvProjects','Select$0')">View</a></td></tr>
               <tr><td>2</td><td>Road, ward 4</td><td>12,50,000.00</td><td></td><td></td></tr>
               <tr><td></td><td>Total</td><td>1750000</td><td>321848</td><td></td></tr>"#,
            r#"<tr><td colspan="5"><table><tr><td><span>1</span></td><td><a href="javascript:__doPostBack('gvProjects','Page$2')">2</a></td></tr></table></td></tr>"#,
        );
        let (rows, pages) = projects(&html).unwrap();
        assert_eq!(pages, vec![2]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], PlanProject { number: "1".into(), name: "പദ്ധതി നിർവഹണ മോണിട്ടറിംഗ് ചെലവുകൾ".into(), planned: Some(500_000), spent: Some(321_848) });
        assert_eq!(rows[1].planned, Some(1_250_000));
        assert_eq!(rows[1].spent, None);
    }

    #[test]
    fn a_changed_table_is_refused() {
        let html = r#"<table id="gvProjects"><tr><th>Sl No</th><th>Project Name</th><th>Amount</th></tr></table>"#;
        assert_eq!(projects(html), Err(Error::MissingColumn("Formulation")));
        assert_eq!(projects("<p>Session expired</p>"), Err(Error::MissingTable("gvProjects")));
    }

    #[test]
    fn targets_come_from_the_setting() {
        assert_eq!(targets(""), vec![]);
        assert_eq!(
            targets("ernakulam:gp, Ernakulam:M,Nowhere:gp,Kollam:x"),
            vec![("Ernakulam".to_string(), Kind::GramaPanchayat), ("Ernakulam".to_string(), Kind::Municipality)]
        );
    }

    #[test]
    fn the_cursor_moves_through_local_bodies_then_targets() {
        let mut c = Cursor::default();
        c.advance(2);
        assert_eq!((c.target, c.local_body), (0, 1));
        c.advance(2);
        assert_eq!((c.target, c.local_body), (1, 0));
        assert!(!c.finished(2));
        c.advance(1);
        assert!(c.finished(2));
    }

    #[test]
    fn the_extract_is_one_line_per_project() {
        let p = [PlanProject { number: "1".into(), name: "a\tb".into(), planned: Some(5), spent: None }];
        let tsv = extract("29", "Ernakulam", Kind::GramaPanchayat, "Alangad", &p);
        assert!(tsv.ends_with("number\tname\tplanned\tspent\n1\ta b\t5\t\n"));
        assert!(tsv.contains("Ernakulam, gp, Alangad"));
    }
}
