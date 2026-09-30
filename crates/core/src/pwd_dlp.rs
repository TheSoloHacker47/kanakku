//! Parser for the Kerala PWD defect-liability list
//! (`https://www.pwd.kerala.gov.in/IMF_website/Projects/wings_list.php`).
//!
//! After a work is finished its contractor stays liable for repairs for a set period. PWD lists
//! those works per wing, fifty to a page, with the contractor's name and the dates. The page
//! also prints contact numbers; we never read those into a record.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Date;

pub const SOURCE_URL: &str = "https://www.pwd.kerala.gov.in/IMF_website/Projects/wings_list.php";
const LISTING_URL: &str = "https://www.pwd.kerala.gov.in/IMF_website/Projects/work_listing_dlp.php";

/// PWD's number for each wing, and the name it gives it.
pub const WINGS: [(u8, &str); 7] =
    [(5, "Roads"), (3, "National Highways"), (4, "Buildings"), (12, "Bridges"), (8, "KRFB-PMU"), (2, "KSTP"), (9, "RICK")];

#[derive(Debug, PartialEq)]
pub enum ParseError {
    NoWorkTable,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no work table in the PWD liability page")
    }
}

impl std::error::Error for ParseError {}

/// A finished work whose contractor is still liable for defects.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LiabilityWork {
    pub wing: String,
    pub name: String,
    pub contractor: Option<String>,
    pub starts_on: Option<Date>,
    pub ends_on: Option<Date>,
    pub division: Option<String>,
    pub subdivision: Option<String>,
}

impl LiabilityWork {
    /// PWD gives works no identifier. These four fields together tell one work from another.
    pub fn identity(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.wing,
            self.name,
            self.starts_on.map(Date::to_iso).unwrap_or_default(),
            self.contractor.as_deref().map(contractor_key).unwrap_or_default()
        )
    }

    /// Days of liability left on `today`; negative once it has ended.
    pub fn days_left(&self, today: Date) -> Option<i32> {
        self.ends_on.map(|end| end.days_since(today))
    }
}

#[derive(Debug, PartialEq)]
pub struct Listing {
    pub works: Vec<LiabilityWork>,
    pub last_page: u32,
}

/// The address of one page of a wing's list, optionally for one division.
/// These are the links the site's own pager uses.
pub fn listing_url(wing: u8, division: Option<&str>, page: u32) -> String {
    let mut url = format!("{LISTING_URL}?wing_id={}", base64(wing.to_string().as_bytes()));
    if let Some(division) = division {
        url.push_str("&division_off=");
        // Written exactly as the site's pager writes it; the site rejects a percent-encoded value.
        url.push_str(&base64(division.as_bytes()));
    }
    url.push_str(&format!("&Page={}", page.max(1)));
    url
}

/// The divisions a wing's page offers in its filter, exactly as spelled there.
/// PWD spells some divisions more than one way, and each spelling holds different works.
pub fn divisions(page: &str) -> Vec<String> {
    let Some(start) = page.find("id=\"division_off\"") else { return Vec::new() };
    let select = &page[start..];
    let select = &select[..select.find("</select>").unwrap_or(select.len())];
    let mut out = Vec::new();
    for option in select.split("<option").skip(1) {
        let Some(value) = option.split("value=\"").nth(1).and_then(|rest| rest.split('"').next()) else { continue };
        let value = decode(value);
        if !value.trim().is_empty() && value != "0" && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// Whether a division serves the district. PWD names divisions after their town.
pub fn in_district(division: &str, district: &str) -> bool {
    let towns: &[&str] = match district.trim().to_ascii_lowercase().as_str() {
        "ernakulam" => &["ernakulam", "muvattupuzha", "muvatupuzha", "aluva"],
        _ => return division.to_ascii_lowercase().contains(&district.trim().to_ascii_lowercase()),
    };
    let division = division.to_ascii_lowercase();
    towns.iter().any(|town| division.contains(town))
}

/// Reads one page of a wing's list.
pub fn parse_listing(page: &str, wing: &str) -> Result<Listing, ParseError> {
    let Some(start) = page.find("id=\"dispDMSresult\"") else {
        // A division with no works is served without the table, but still with the filter.
        return if page.contains("id=\"division_off\"") { Ok(Listing { works: Vec::new(), last_page: 1 }) } else { Err(ParseError::NoWorkTable) };
    };
    let body = &page[start..];
    let body = strip_comments(&body[..body.find("</table>").unwrap_or(body.len())]);

    let mut works = Vec::new();
    for row in body.split("<tr").skip(1) {
        let cells = cells(row);
        // number, work, start, end, contractor, contractor's phone, division, subdivision, engineer's phone
        if let [number, name, start, end, contractor, _, division, subdivision, ..] = cells.as_slice() {
            if number.parse::<u32>().is_ok() && !name.is_empty() {
                works.push(LiabilityWork {
                    wing: wing.to_string(),
                    name: name.clone(),
                    contractor: some(contractor),
                    starts_on: date(start),
                    ends_on: date(end),
                    division: some(division),
                    subdivision: some(subdivision),
                });
            }
        }
    }

    let last_page = body
        .split("Page=")
        .skip(1)
        .filter_map(|rest| rest.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u32>().ok())
        .max()
        .unwrap_or(1);
    Ok(Listing { works, last_page })
}

/// A key that treats "Shri. P.V. Stephan" and "P V STEPHAN" as the same contractor.
pub fn contractor_key(name: &str) -> String {
    const TITLES: [&str; 9] = ["shri", "sri", "smt", "mr", "mrs", "ms", "m/s", "m/s.", "messrs"];
    name.to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '.' || c == ',')
        .filter(|word| !word.is_empty() && !TITLES.contains(word))
        .flat_map(|word| word.chars().filter(|c| c.is_alphanumeric()))
        .collect()
}

/// A contractor's name for display: titles dropped, shouting capitals calmed, initials kept.
pub fn contractor_display(name: &str) -> String {
    const TITLES: [&str; 6] = ["shri", "sri", "smt", "mr", "mrs", "ms"];
    let words: Vec<&str> = name
        .split_whitespace()
        .filter(|word| !TITLES.contains(&word.trim_end_matches('.').to_lowercase().as_str()))
        .collect();
    let letters = words.iter().flat_map(|w| w.chars()).filter(|c| c.is_alphabetic()).count();
    let capitals = words.iter().flat_map(|w| w.chars()).filter(|c| c.is_uppercase()).count();
    let shouting = letters > 3 && capitals * 10 >= letters * 8;
    words
        .iter()
        .map(|word| {
            if !shouting || word.chars().filter(|c| c.is_alphabetic()).count() <= 2 || word.eq_ignore_ascii_case("M/s") {
                return word.to_string();
            }
            let mut out = String::with_capacity(word.len());
            let mut first = true;
            for c in word.chars() {
                if first && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    first = false;
                } else {
                    out.extend(c.to_lowercase());
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A work's name without PWD's filing prefix and suffix:
/// "GENERAL-Restoration of X-WORK-General Civil Work" becomes "Restoration of X".
pub fn work_display(name: &str) -> &str {
    const SUFFIX: &str = "general civil work";
    let mut name = name.trim();
    if let Some(at) = name.rfind("-WORK-").filter(|at| *at > 0) {
        name = &name[..at];
    }
    loop {
        let trimmed = name.trim_end_matches(|c: char| c == '-' || c.is_whitespace());
        let cut = trimmed.len().saturating_sub(SUFFIX.len());
        if trimmed.len() > SUFFIX.len() && trimmed.is_char_boundary(cut) && trimmed[cut..].eq_ignore_ascii_case(SUFFIX) {
            name = &trimmed[..cut];
        } else {
            name = trimmed;
            break;
        }
    }
    name.strip_prefix("GENERAL-").unwrap_or(name).trim()
}

fn date(s: &str) -> Option<Date> {
    Date::parse_dmy(&s.trim().replace(['.', '/'], "-"))
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find("<!--") {
        out.push_str(&rest[..open]);
        match rest[open..].find("-->") {
            Some(close) => rest = &rest[open + close + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
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
    decode(&plain).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode(s: &str) -> String {
    s.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'")
}

fn some(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    const ROADS: &str = include_str!("../tests/fixtures/pwd_dlp_roads.html");
    const BUILDINGS: &str = include_str!("../tests/fixtures/pwd_dlp_buildings_paged.html");

    #[test]
    fn reads_the_rows_and_skips_commented_out_columns() {
        let listing = parse_listing(ROADS, "Roads").unwrap();
        assert_eq!(listing.works.len(), 8);
        assert_eq!(listing.last_page, 1);
        assert_eq!(
            listing.works[0],
            LiabilityWork {
                wing: "Roads".into(),
                name: "GENERAL-Restoration Works 2020-21: Surface Rectification and other works in Karukutty Azhakam Road ch 0/000 to 2/000 and Thuravoor Mookanoor Road 0/000 to ch 2/000-WORK-General Civil Work".into(),
                contractor: Some("REGI T I".into()),
                starts_on: Date::parse_iso("2022-01-27"),
                ends_on: Date::parse_iso("2028-02-27"),
                division: Some("Roads Division Ernakulam".into()),
                subdivision: Some("Roads Sub Division Aluva".into()),
            }
        );
    }

    #[test]
    fn never_reads_a_phone_number_into_a_record() {
        let listing = parse_listing(ROADS, "Roads").unwrap();
        let stored = serde_json::to_string(&listing.works).unwrap();
        assert!(!stored.contains("0000000000"), "the fixture's stand-in phone number reached a record");
    }

    #[test]
    fn finds_the_last_page_from_the_pager() {
        let listing = parse_listing(BUILDINGS, "Buildings").unwrap();
        assert_eq!(listing.works.len(), 4);
        assert_eq!(listing.last_page, 5);
    }

    #[test]
    fn a_page_without_the_table_is_an_error_unless_it_is_an_empty_division() {
        assert_eq!(parse_listing("<html></html>", "Roads").unwrap_err(), ParseError::NoWorkTable);
        let empty = r#"<select name="division_off" id="division_off"><option value="0">Division</option></select>"#;
        assert_eq!(parse_listing(empty, "KSTP").unwrap(), Listing { works: Vec::new(), last_page: 1 });
    }

    #[test]
    fn lists_divisions_as_spelled_and_picks_the_districts() {
        let all = divisions(ROADS);
        assert!(all.contains(&"Roads Division Ernakulam".to_string()));
        assert!(all.contains(&"Roads Division  Ernakulam".to_string()), "the double-space spelling is a separate division");
        assert!(!all.iter().any(|d| d == "0" || d.trim().is_empty()));

        let mine: Vec<&String> = all.iter().filter(|d| in_district(d, "Ernakulam")).collect();
        assert!(mine.iter().any(|d| d.contains("Muvattupuzha")));
        assert!(mine.iter().any(|d| d.contains("Muvatupuzha")));
        assert!(!mine.iter().any(|d| d.contains("Thrissur")));
        assert!(in_district("Roads Division Thrissur", "Thrissur"));
    }

    #[test]
    fn builds_the_pager_links_the_site_uses() {
        assert_eq!(listing_url(4, None, 1), format!("{LISTING_URL}?wing_id=NA==&Page=1"));
        assert_eq!(
            listing_url(4, Some("Buildings Division Ernakulam"), 2),
            format!("{LISTING_URL}?wing_id=NA==&division_off=QnVpbGRpbmdzIERpdmlzaW9uIEVybmFrdWxhbQ==&Page=2")
        );
        assert_eq!(listing_url(12, None, 0), format!("{LISTING_URL}?wing_id=MTI=&Page=1"));
    }

    #[test]
    fn reads_dates_with_dots_or_slashes() {
        assert_eq!(date("01.10.2021"), Date::parse_iso("2021-10-01"));
        assert_eq!(date(" 05/10/2026 "), Date::parse_iso("2026-10-05"));
        assert_eq!(date("soon"), None);
    }

    #[test]
    fn contractor_names_group_and_display() {
        assert_eq!(contractor_key("Shri. P.V. Stephan"), contractor_key("P V STEPHAN"));
        assert_eq!(contractor_key("M/s Chemparaky LCS"), "chemparakylcs");
        assert_ne!(contractor_key("P V Stephan"), contractor_key("P V Stephen"));

        assert_eq!(contractor_display("P V STEPHAN"), "P V Stephan");
        assert_eq!(contractor_display("Shri. A. ABDUL HAKKIM"), "A. Abdul Hakkim");
        assert_eq!(contractor_display("Baiju A A"), "Baiju A A");
        assert_eq!(contractor_display("M/s Chemparaky LCS"), "M/s Chemparaky LCS");
    }

    #[test]
    fn work_names_lose_the_filing_prefix_and_suffix() {
        assert_eq!(work_display("GENERAL-Restoration of X road-WORK-General Civil Work"), "Restoration of X road");
        assert_eq!(work_display("Construction of new block"), "Construction of new block");
        assert_eq!(
            work_display("Harbour bridge - Improvements of Approach Road-Approach road-General Civil Work-General Civil Work-"),
            "Harbour bridge - Improvements of Approach Road-Approach road"
        );
        assert_eq!(work_display("General Civil Work"), "General Civil Work");
    }

    #[test]
    fn counts_days_left() {
        let work = LiabilityWork { ends_on: Date::parse_iso("2026-10-05"), ..LiabilityWork::default() };
        assert_eq!(work.days_left(Date::parse_iso("2026-09-30").unwrap()), Some(5));
        assert_eq!(work.days_left(Date::parse_iso("2026-10-06").unwrap()), Some(-1));
    }
}
