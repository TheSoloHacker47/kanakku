//! Parser for the Kerala PWD defect-liability list
//! (`https://www.pwd.kerala.gov.in/IMF_website/Projects/wings_list.php`).
//!
//! After a work is finished its contractor stays liable for repairs for a set period. PWD lists
//! those works per wing, fifty to a page, with the contractor's name and the dates. The page
//! also prints contact numbers; we never read those into a record.
//!
//! The agreed contract amount is in each row's markup but commented out, so PWD's page does not
//! display it. We read it. The contractor's address, commented out the same way, we do not.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::entity;
use crate::names::canonical_district;
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
    /// The agreed contract amount in whole rupees, where PWD's markup carries one.
    #[serde(default)]
    pub agreed_amount: Option<i64>,
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
            self.contractor.as_deref().map(identity_key).unwrap_or_default()
        )
    }

    /// The contractor's key, shared with the other sources.
    pub fn contractor_key(&self) -> Option<String> {
        self.contractor.as_deref().map(entity::contractor_key).filter(|key| !key.is_empty())
    }

    /// The district of the PWD office that handles the work, from the names of its subdivision
    /// and division. A division can cover works outside its own district.
    pub fn office_district(&self) -> Option<&'static str> {
        self.subdivision.as_deref().and_then(district_of).or_else(|| self.division.as_deref().and_then(district_of))
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

/// Reads one page of a wing's list.
pub fn parse_listing(page: &str, wing: &str) -> Result<Listing, ParseError> {
    let Some(start) = page.find("id=\"dispDMSresult\"") else {
        // A division with no works is served without the table, but still with the filter.
        return if page.contains("id=\"division_off\"") { Ok(Listing { works: Vec::new(), last_page: 1 }) } else { Err(ParseError::NoWorkTable) };
    };
    let body = &page[start..];
    let body = &body[..body.find("</table>").unwrap_or(body.len())];

    let mut works = Vec::new();
    for raw_row in body.split("<tr").skip(1) {
        let agreed_amount = commented_amount(raw_row);
        let row = strip_comments(raw_row);
        let cells = cells(&row);
        // number, work, start, end, contractor, contractor's phone, division, subdivision, engineer's phone
        if let [number, name, start, end, contractor, _, division, subdivision, ..] = cells.as_slice() {
            if number.parse::<u32>().is_ok() && !name.is_empty() {
                works.push(LiabilityWork {
                    wing: wing.to_string(),
                    name: name.clone(),
                    contractor: some(contractor),
                    agreed_amount,
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

/// The contractor part of a work's identity. Kept as first written: changing it would give
/// every stored work a new identity. Grouping across sources uses `entity::contractor_key`.
fn identity_key(name: &str) -> String {
    const TITLES: [&str; 9] = ["shri", "sri", "smt", "mr", "mrs", "ms", "m/s", "m/s.", "messrs"];
    name.to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '.' || c == ',')
        .filter(|word| !word.is_empty() && !TITLES.contains(word))
        .flat_map(|word| word.chars().filter(|c| c.is_alphanumeric()))
        .collect()
}

/// Towns PWD names offices after, and the district each is in.
const TOWNS: &[(&str, &str)] = &[
    ("muvattupuzha", "Ernakulam"), ("muvatupuzha", "Ernakulam"), ("aluva", "Ernakulam"), ("paravur", "Ernakulam"),
    ("perumbavoor", "Ernakulam"), ("kothamangalam", "Ernakulam"), ("piravom", "Ernakulam"), ("kochi", "Ernakulam"),
    ("thalassery", "Kannur"), ("taliparamba", "Kannur"), ("iritty", "Kannur"), ("payyannur", "Kannur"),
    ("manjeri", "Malappuram"), ("tirur", "Malappuram"), ("perinthalmanna", "Malappuram"), ("nilambur", "Malappuram"), ("ponnani", "Malappuram"),
    ("kodungallur", "Thrissur"), ("irinjalakuda", "Thrissur"), ("chalakudy", "Thrissur"), ("chalakkudy", "Thrissur"), ("kunnamkulam", "Thrissur"),
    ("shornur", "Palakkad"), ("shoranur", "Palakkad"), ("ottapalam", "Palakkad"), ("ottappalam", "Palakkad"), ("chittur", "Palakkad"), ("mannarkkad", "Palakkad"), ("alathur", "Palakkad"),
    ("vadakara", "Kozhikode"), ("vatakara", "Kozhikode"), ("koyilandy", "Kozhikode"), ("thamarassery", "Kozhikode"),
    ("kalpetta", "Wayanad"), ("mananthavady", "Wayanad"), ("bathery", "Wayanad"),
    ("kanhangad", "Kasaragod"),
    ("pala", "Kottayam"), ("kanjirappally", "Kottayam"), ("changanassery", "Kottayam"), ("vaikom", "Kottayam"), ("gandhinagar", "Kottayam"),
    ("thodupuzha", "Idukki"), ("kattappana", "Idukki"), ("devikulam", "Idukki"), ("peerumade", "Idukki"),
    ("chengannur", "Alappuzha"), ("haripad", "Alappuzha"), ("cherthala", "Alappuzha"), ("cherthla", "Alappuzha"), ("mavelikkara", "Alappuzha"), ("kayamkulam", "Alappuzha"), ("kuttanad", "Alappuzha"),
    ("adoor", "Pathanamthitta"), ("thiruvalla", "Pathanamthitta"), ("ranni", "Pathanamthitta"), ("konni", "Pathanamthitta"),
    ("punalur", "Kollam"), ("kottarakkara", "Kollam"), ("karunagappally", "Kollam"), ("karunagapally", "Kollam"),
    ("neyyattinkara", "Thiruvananthapuram"), ("attingal", "Thiruvananthapuram"), ("nedumangad", "Thiruvananthapuram"), ("kazhakuttom", "Thiruvananthapuram"),
    ("secretariate", "Thiruvananthapuram"), ("legislative complex", "Thiruvananthapuram"), ("klc", "Thiruvananthapuram"), ("vazhuthacaud", "Thiruvananthapuram"),
];

/// The district an office name points to: a district's own name or spelling, or a town in it.
pub fn district_of(office: &str) -> Option<&'static str> {
    let lower = office.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    if let Some(district) = words.iter().find_map(|word| canonical_district(word)) {
        return Some(district);
    }
    TOWNS.iter().find(|(town, _)| if town.contains(' ') { lower.contains(town) } else { words.contains(town) }).map(|(_, district)| *district)
}

/// Whether an office serves the district, or any district when `district` stands for all.
pub fn in_district(office: &str, district: &str) -> bool {
    district == crate::names::ALL_DISTRICTS || district_of(office).is_some_and(|d| d.eq_ignore_ascii_case(district.trim()))
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

/// The agreed amount, which sits in the first commented-out cell of a row, right after the work's name.
fn commented_amount(row: &str) -> Option<i64> {
    let comment = &row[row.find("<!--")? + 4..];
    let comment = &comment[..comment.find("-->")?];
    let value: f64 = cells(comment).first()?.replace(',', "").parse().ok()?;
    (value.is_finite() && value > 0.0).then(|| value.round() as i64)
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
    fn reads_the_rows_and_the_commented_out_amount() {
        let listing = parse_listing(ROADS, "Roads").unwrap();
        assert_eq!(listing.works.len(), 8);
        assert_eq!(listing.last_page, 1);
        assert_eq!(
            listing.works[0],
            LiabilityWork {
                wing: "Roads".into(),
                name: "GENERAL-Restoration Works 2020-21: Surface Rectification and other works in Karukutty Azhakam Road ch 0/000 to 2/000 and Thuravoor Mookanoor Road 0/000 to ch 2/000-WORK-General Civil Work".into(),
                contractor: Some("REGI T I".into()),
                agreed_amount: Some(2_777_707),
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
    fn a_blank_or_zero_amount_is_not_an_amount() {
        let row = |comment: &str| format!("<td>1</td><td>Work</td>{comment}<td>01/01/2024</td>");
        assert_eq!(commented_amount(&row(r#"<!--<td width="25%">4371925.85</td>-->"#)), Some(4_371_926));
        assert_eq!(commented_amount(&row(r#"<!--<td width="25%"></td>-->"#)), None);
        assert_eq!(commented_amount(&row(r#"<!--<td width="25%">0</td>-->"#)), None);
        assert_eq!(commented_amount(&row("")), None);
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
        assert!(in_district("Roads Division Thrissur", crate::names::ALL_DISTRICTS));
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
    fn offices_resolve_to_districts() {
        assert_eq!(district_of("Roads Division Muvatupuzha"), Some("Ernakulam"));
        assert_eq!(district_of("Buildings Division,Kasaragod"), Some("Kasaragod"));
        assert_eq!(district_of("KRFB-PMU Division Kasragod"), Some("Kasaragod"));
        assert_eq!(district_of("Electrical Division TVM"), Some("Thiruvananthapuram"));
        assert_eq!(district_of("Buildings Division Thalassery"), Some("Kannur"));
        assert_eq!(district_of("Roads Division Manjeri"), Some("Malappuram"));
        assert_eq!(district_of("Buildings Sub Division Shornur"), Some("Palakkad"));
        assert_eq!(district_of("Special Buildings Sub Division, Kottayam"), Some("Kottayam"));
        assert_eq!(district_of("Pala"), Some("Kottayam"));
        assert_eq!(district_of("Palakkad"), Some("Palakkad"), "a town name inside a longer word is not a match");
        assert_eq!(district_of("Legislative Complex Construction Division "), Some("Thiruvananthapuram"));
        assert_eq!(district_of("Central Stores"), None);

        let work = LiabilityWork {
            division: Some("Bridges Division Ernakulam".into()),
            subdivision: Some("Bridges Sub Division Thrissur".into()),
            ..LiabilityWork::default()
        };
        assert_eq!(work.office_district(), Some("Thrissur"), "the subdivision is nearer the work than the division");
    }

    #[test]
    fn identity_does_not_change_with_the_shared_contractor_key() {
        let work = LiabilityWork {
            wing: "Roads".into(),
            name: "Road".into(),
            contractor: Some("M/s Uralungal Society Ltd".into()),
            starts_on: Date::parse_iso("2022-01-27"),
            ..LiabilityWork::default()
        };
        assert_eq!(work.identity(), "Roads|Road|2022-01-27|uralungalsocietyltd");
        assert_eq!(work.contractor_key().as_deref(), Some("uralungalsociety"));
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
