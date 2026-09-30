//! Parser for the KIIFB integrated dashboard (`https://gis.kiifb.org/`).
//!
//! The page embeds its whole dataset as two JavaScript constants:
//! `MARKERS` (project pins) and `TRANSPORT_GEOJSON` (road and bridge works).
//! We slice those JSON values out of the page and read them with serde.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops::Range;

use serde::de::{self, DeserializeOwned, Deserializer, IgnoredAny, SeqAccess, Visitor};
use serde::Deserialize;

use crate::model::{Constituency, Project, Site, Work};
use crate::names::{canonical_constituency, constituency_ml};
use crate::Date;

pub const SOURCE_URL: &str = "https://gis.kiifb.org/";

/// Bump when the parser reads the same page differently, so stored records are rebuilt.
pub const PARSER_VERSION: u32 = 2;

const RUPEES_PER_CRORE: f64 = 10_000_000.0;

#[derive(Debug)]
pub enum ParseError {
    SegmentMissing(&'static str),
    Json(&'static str, serde_json::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::SegmentMissing(name) => write!(f, "`const {name}` not found in the KIIFB page"),
            ParseError::Json(name, e) => write!(f, "`{name}` is not the JSON we expect: {e}"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub struct Parsed {
    /// Projects in the requested district, sorted by code.
    pub projects: Vec<Project>,
    /// Pins and works in the whole page, before the district filter.
    pub markers_total: usize,
    pub works_total: usize,
    /// Byte ranges of the two JSON values, so callers can hash only the data.
    pub segments: [Range<usize>; 2],
}

/// Reads the dashboard page and returns the projects of one district.
pub fn parse(page: &[u8], district: &str) -> Result<Parsed, ParseError> {
    let (markers, markers_range) = segment::<Vec<MarkerRaw>>(page, "MARKERS")?;
    let (transport, transport_range) = segment::<FeatureCollection>(page, "TRANSPORT_GEOJSON")?;
    let markers_total = markers.len();
    let works_total = transport.features.len();

    // The estimate is stated per sub-project and repeated on each of its packages, anywhere in
    // the state. Count the packages behind each (sub-project, estimate) pair and add up their spending.
    let mut groups: HashMap<(String, i64), Group> = HashMap::new();
    for m in &markers {
        let (Some(code), Some(key)) = (m.proj.code.as_ref(), group_key(&m.proj)) else { continue };
        let group = groups.entry(key).or_default();
        if !group.codes.contains(code) {
            group.codes.push(code.clone());
            group.expenditure += rupees(m.proj.expenditure, 1.0).unwrap_or(0);
        }
    }

    let mut projects: BTreeMap<String, Project> = BTreeMap::new();

    for m in markers {
        if !same_district(&m.d, district) {
            continue;
        }
        let Some(code) = m.proj.code.clone() else { continue };
        let project = projects.entry(code.clone()).or_insert_with(|| {
            let mut project = project_from_marker(code, &m, district);
            match group_key(&m.proj).and_then(|key| groups.get(&key)) {
                Some(group) => {
                    project.estimate_shared_by = group.codes.len() as u32;
                    project.group_expenditure = (group.expenditure > 0).then_some(group.expenditure);
                }
                None => project.estimate_shared_by = 1,
            }
            project
        });
        if let Some(name) = m.proj.constituency.clone().or_else(|| clean(&m.c)) {
            let link = m.link.as_ref();
            add_constituency(
                project,
                Constituency {
                    name,
                    name_ml: link.and_then(|l| l.constituency_ml.as_deref()).and_then(strip_number),
                    mla_name: link.and_then(|l| l.name.clone()),
                    mla_name_ml: link.and_then(|l| l.name_ml.clone()),
                },
            );
        }
        if let (Some(lat), Some(lng)) = (m.lat, m.lng) {
            let site = Site { lat, lng };
            if !project.sites.contains(&site) {
                project.sites.push(site);
            }
        }
    }

    for f in transport.features {
        let p = f.properties;
        if !p.district.as_deref().is_some_and(|d| same_district(d, district)) {
            continue;
        }
        let Some(code) = p.code.clone() else { continue };
        let mid = f.geometry.and_then(|g| g.coordinates.0);
        let project = projects.entry(code.clone()).or_insert_with(|| project_from_work(code, &p, district));
        if let Some(name) = p.lac.clone() {
            add_constituency(project, Constituency { name, ..Constituency::default() });
        }
        let work = work_from_props(p, mid);
        match project.works.iter_mut().find(|known| work.is_segment_of(known)) {
            Some(known) => {
                if known.road_name.is_none() {
                    known.road_name = work.road_name;
                }
            }
            None => project.works.push(work),
        }
    }

    Ok(Parsed {
        projects: projects.into_values().collect(),
        markers_total,
        works_total,
        segments: [markers_range, transport_range],
    })
}

/// Finds `const NAME = <json>` in the page and deserialises the JSON value.
fn segment<T: DeserializeOwned>(page: &[u8], name: &'static str) -> Result<(T, Range<usize>), ParseError> {
    let needle = format!("const {name}");
    let mut from = 0;
    let start = loop {
        let at = from + memchr::memmem::find(&page[from..], needle.as_bytes()).ok_or(ParseError::SegmentMissing(name))?;
        let mut i = at + needle.len();
        while page.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        // Skip longer identifiers that merely start with the name.
        if page.get(i) == Some(&b'=') {
            i += 1;
            while page.get(i).is_some_and(u8::is_ascii_whitespace) {
                i += 1;
            }
            break i;
        }
        from = at + needle.len();
    };
    let mut stream = serde_json::Deserializer::from_slice(&page[start..]).into_iter::<T>();
    let value = stream
        .next()
        .ok_or(ParseError::SegmentMissing(name))?
        .map_err(|e| ParseError::Json(name, e))?;
    Ok((value, start..start + stream.byte_offset()))
}

fn same_district(value: &str, district: &str) -> bool {
    value.trim().eq_ignore_ascii_case(district)
}

#[derive(Default)]
struct Group {
    codes: Vec<String>,
    expenditure: i64,
}

fn group_key(proj: &ProjRaw) -> Option<(String, i64)> {
    Some((proj.sub_code.clone()?, rupees(proj.estimated_amount, 1.0)?))
}

/// Adds a constituency under its canonical name. A marker's entry carries the MLA, a work's
/// only the name, so a later entry fills in whatever an earlier one lacked.
fn add_constituency(project: &mut Project, mut c: Constituency) {
    c.name = canonical_constituency(&c.name);
    if c.name.is_empty() {
        return;
    }
    if let Some(ml) = constituency_ml(&c.name) {
        c.name_ml = Some(ml.to_string());
    }
    match project.constituencies.iter_mut().find(|known| known.name == c.name) {
        Some(known) => {
            known.name_ml = known.name_ml.take().or(c.name_ml);
            known.mla_name = known.mla_name.take().or(c.mla_name);
            known.mla_name_ml = known.mla_name_ml.take().or(c.mla_name_ml);
        }
        None => project.constituencies.push(c),
    }
}

/// "എറണാകുളം (82)" becomes "എറണാകുളം".
fn strip_number(name: &str) -> Option<String> {
    clean(name.split('(').next().unwrap_or(name))
}

fn project_from_marker(code: String, m: &MarkerRaw, district: &str) -> Project {
    Project {
        code,
        title: m.proj.name.clone().unwrap_or_default(),
        department: m.dept_excel.clone().or_else(|| m.proj.department.clone()),
        sector: m.dept_sector.clone().or_else(|| m.proj.sector.clone()),
        executing_agency: m.proj.executing_authority.clone(),
        district: district.to_string(),
        // Labelled "(Cr)" on the dashboard, but the values are rupees.
        estimated_amount: rupees(m.proj.estimated_amount, 1.0),
        sub_project_code: m.proj.sub_code.clone(),
        estimate_shared_by: 1,
        group_expenditure: None,
        expenditure: rupees(m.proj.expenditure, 1.0),
        status: m.proj.status.clone().or_else(|| m.s.clone()),
        ..Project::default()
    }
}

fn project_from_work(code: String, p: &PropsRaw, district: &str) -> Project {
    Project {
        code,
        title: p.name.clone().unwrap_or_default(),
        department: p.department.clone(),
        executing_agency: p.spv.clone(),
        district: district.to_string(),
        ..Project::default()
    }
}

fn work_from_props(p: PropsRaw, mid: Option<(f64, f64)>) -> Work {
    let crore = |v| rupees(v, RUPEES_PER_CRORE);
    Work {
        road_name: p.road_name,
        spv: p.spv,
        contractor: p.contractor,
        as_amount: crore(p.as_amount),
        fs_amount: crore(p.fs_amount),
        ts_amount: crore(p.ts_amount),
        tender_amount: crore(p.tender_amount),
        loa_amount: crore(p.loa_amount),
        contract_amount: crore(p.contract_amount),
        paid_amount: crore(p.paid_amount),
        paid_contractor: crore(p.paid_contractor),
        scheduled_start: p.start.as_deref().and_then(Date::parse_dmy),
        scheduled_end: p.finish.as_deref().and_then(Date::parse_dmy),
        progress_note: p.progress_note,
        physical_pct: p.physical_pct,
        financial_pct: p.financial_pct,
        status: p.status,
        lng: mid.map(|m| m.0),
        lat: mid.map(|m| m.1),
    }
}

/// Converts to whole rupees. KIIFB writes 0 where it has no figure, so 0 becomes `None`.
fn rupees(value: Option<f64>, scale: f64) -> Option<i64> {
    let v = (value? * scale).round();
    (v.is_finite() && v > 0.0).then_some(v as i64)
}

/// Trims, collapses whitespace (including the non-breaking spaces KIIFB uses) and drops empties.
fn clean(s: &str) -> Option<String> {
    let out = s.split(|c: char| c.is_whitespace() || c == '\u{a0}').filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ");
    (!out.is_empty()).then_some(out)
}

// ---- raw shapes -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct MarkerRaw {
    #[serde(default, deserialize_with = "num")]
    lat: Option<f64>,
    #[serde(default, deserialize_with = "num")]
    lng: Option<f64>,
    #[serde(default)]
    d: String,
    #[serde(default)]
    c: String,
    #[serde(default, deserialize_with = "text")]
    s: Option<String>,
    #[serde(default, deserialize_with = "text")]
    dept_excel: Option<String>,
    #[serde(default, deserialize_with = "text")]
    dept_sector: Option<String>,
    proj: ProjRaw,
    #[serde(default, deserialize_with = "link")]
    link: Option<LinkRaw>,
}

#[derive(Deserialize)]
struct ProjRaw {
    #[serde(rename = "Project Name", default, deserialize_with = "text")]
    name: Option<String>,
    #[serde(rename = "Estimated Amount (Cr)", default, deserialize_with = "num")]
    estimated_amount: Option<f64>,
    #[serde(rename = "Expenditure", default, deserialize_with = "num")]
    expenditure: Option<f64>,
    #[serde(rename = "Status", default, deserialize_with = "text")]
    status: Option<String>,
    #[serde(rename = "Constituency", default, deserialize_with = "text")]
    constituency: Option<String>,
    #[serde(rename = "Executing Authority", default, deserialize_with = "text")]
    executing_authority: Option<String>,
    #[serde(rename = "Department", default, deserialize_with = "text")]
    department: Option<String>,
    #[serde(rename = "Sector Dept", default, deserialize_with = "text")]
    sector: Option<String>,
    #[serde(rename = "Sub Proj Code", default, deserialize_with = "text")]
    sub_code: Option<String>,
    #[serde(rename = "Proj Code", default, deserialize_with = "text")]
    code: Option<String>,
}

#[derive(Deserialize)]
struct LinkRaw {
    #[serde(rename = "Name", default, deserialize_with = "text")]
    name: Option<String>,
    #[serde(rename = "മണ്ഡലം", default, deserialize_with = "text")]
    constituency_ml: Option<String>,
    #[serde(rename = "പേര്", default, deserialize_with = "text")]
    name_ml: Option<String>,
}

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    properties: PropsRaw,
    #[serde(default)]
    geometry: Option<Geometry>,
}

#[derive(Deserialize)]
struct Geometry {
    coordinates: MidPoint,
}

#[derive(Deserialize)]
struct PropsRaw {
    #[serde(rename = "Department", default, deserialize_with = "text")]
    department: Option<String>,
    #[serde(rename = "SPV", default, deserialize_with = "text")]
    spv: Option<String>,
    #[serde(rename = "Proj_Code", default, deserialize_with = "text")]
    code: Option<String>,
    #[serde(rename = "Proj_Name", default, deserialize_with = "text")]
    name: Option<String>,
    #[serde(rename = "District", default, deserialize_with = "text")]
    district: Option<String>,
    #[serde(rename = "LAC", default, deserialize_with = "text")]
    lac: Option<String>,
    #[serde(rename = "AS_Amount", default, deserialize_with = "num")]
    as_amount: Option<f64>,
    #[serde(rename = "FS_Amount", default, deserialize_with = "num")]
    fs_amount: Option<f64>,
    #[serde(rename = "TS_Amount", default, deserialize_with = "num")]
    ts_amount: Option<f64>,
    #[serde(rename = "TenderAmnt", default, deserialize_with = "num")]
    tender_amount: Option<f64>,
    #[serde(rename = "LOA_Amount", default, deserialize_with = "num")]
    loa_amount: Option<f64>,
    #[serde(rename = "Contr_Amnt", default, deserialize_with = "num")]
    contract_amount: Option<f64>,
    #[serde(rename = "Paid_Amnt", default, deserialize_with = "num")]
    paid_amount: Option<f64>,
    #[serde(rename = "Paid_Contr", default, deserialize_with = "num")]
    paid_contractor: Option<f64>,
    #[serde(rename = "Contractor", default, deserialize_with = "text")]
    contractor: Option<String>,
    #[serde(rename = "Start_Dt", default, deserialize_with = "text")]
    start: Option<String>,
    #[serde(rename = "Finish_Dt", default, deserialize_with = "text")]
    finish: Option<String>,
    #[serde(rename = "Curnt_Pgrs", default, deserialize_with = "text")]
    progress_note: Option<String>,
    #[serde(rename = "Achieved__", default, deserialize_with = "num")]
    physical_pct: Option<f64>,
    #[serde(rename = "Fin_Prgs", default, deserialize_with = "num")]
    financial_pct: Option<f64>,
    #[serde(rename = "Status", default, deserialize_with = "text")]
    status: Option<String>,
    #[serde(rename = "Road_Name", default, deserialize_with = "text")]
    road_name: Option<String>,
}

// ---- lenient field readers ------------------------------------------------------------------
// KIIFB's export is loosely typed: numbers arrive as ints, floats, strings or null.

fn num<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<f64>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a number, numeric string or null")
        }
        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.trim().replace(',', "").parse().ok())
        }
        fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    d.deserialize_any(V)
}

fn text<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a string, number or null")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            Ok(clean(v))
        }
        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    d.deserialize_any(V)
}

/// `link` is an object when the constituency is known and an empty array when it is not.
fn link<'de, D: Deserializer<'de>>(d: D) -> Result<Option<LinkRaw>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Option<LinkRaw>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an object, array or null")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            LinkRaw::deserialize(de::value::MapAccessDeserializer::new(map)).map(Some)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    d.deserialize_any(V)
}

/// The middle vertex of a MultiLineString's first line, as `(lng, lat)`.
/// Read straight off the token stream: the geometry is three quarters of the page
/// and we only need one point per work.
struct MidPoint(Option<(f64, f64)>);

impl<'de> Deserialize<'de> for MidPoint {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Lines;
        impl<'de> Visitor<'de> for Lines {
            type Value = MidPoint;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("MultiLineString coordinates")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut mid = None;
                while mid.is_none() {
                    match seq.next_element::<Vec<Point>>()? {
                        Some(line) => mid = line.get(line.len() / 2).map(|p| (p.0, p.1)),
                        None => return Ok(MidPoint(None)),
                    }
                }
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(MidPoint(mid))
            }
        }
        d.deserialize_seq(Lines)
    }
}

struct Point(f64, f64);

impl<'de> Deserialize<'de> for Point {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct P;
        impl<'de> Visitor<'de> for P {
            type Value = Point;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a [lng, lat, ...] position")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let lng = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let lat = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(Point(lng, lat))
            }
        }
        d.deserialize_seq(P)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rupees_treats_zero_as_missing() {
        assert_eq!(rupees(Some(0.0), 1.0), None);
        assert_eq!(rupees(None, 1.0), None);
        assert_eq!(rupees(Some(214_300_000.0), 1.0), Some(214_300_000));
        assert_eq!(rupees(Some(86.34), RUPEES_PER_CRORE), Some(863_400_000));
        assert_eq!(rupees(Some(74.96843761), RUPEES_PER_CRORE), Some(749_684_376));
    }

    #[test]
    fn clean_handles_nbsp_and_blanks() {
        assert_eq!(clean("Kunnathunad\u{a0}(SC) ").as_deref(), Some("Kunnathunad (SC)"));
        assert_eq!(clean("  a   b "), Some("a b".into()));
        assert_eq!(clean(" \u{a0} "), None);
    }

    #[test]
    fn segment_skips_longer_identifiers() {
        let page = br#"const MARKERS_COUNT = 9; const MARKERS   = [1,2,3]; const X = 1;"#;
        let (v, range) = segment::<Vec<u8>>(page, "MARKERS").unwrap();
        assert_eq!(v, [1, 2, 3]);
        assert_eq!(&page[range], b"[1,2,3]");
    }

    #[test]
    fn missing_segment_is_an_error() {
        assert!(matches!(parse(b"<html></html>", "Ernakulam"), Err(ParseError::SegmentMissing("MARKERS"))));
    }

    const SMALL_PAGE: &str = r#"<script>
const MARKERS = [
 {"lat":10.0,"lng":76.3,"d":"Ernakulam","c":"Aluva","proj":{"Project Name":"Park, civil","Estimated Amount (Cr)":1000,"Expenditure":100,"Sub Proj Code":"AGR001-01","Proj Code":"AGR001-01-01"},"link":[]},
 {"lat":10.5,"lng":76.2,"d":"Thrissur","c":"Ollur","proj":{"Project Name":"Park, electrical","Estimated Amount (Cr)":1000,"Expenditure":50,"Sub Proj Code":"AGR001-01","Proj Code":"AGR001-01-02"},"link":[]},
 {"lat":10.5,"lng":76.2,"d":"Thrissur","c":"Thrissur","proj":{"Project Name":"Park, electrical","Estimated Amount (Cr)":1000,"Expenditure":50,"Sub Proj Code":"AGR001-01","Proj Code":"AGR001-01-02"},"link":[]},
 {"lat":10.1,"lng":76.4,"d":"Ernakulam","c":"Aluva","proj":{"Project Name":"School","Estimated Amount (Cr)":500,"Sub Proj Code":"EDU002-01","Proj Code":"EDU002-01-01"},"link":[]}
];
const TRANSPORT_GEOJSON = {"type":"FeatureCollection","features":[
 {"type":"Feature","properties":{"Proj_Code":"PWD015-102-02","Proj_Name":"182 Roads","District":"Ernakulam","LAC":"Aluva","FS_Amount":9.5,"Contr_Amnt":9.1,"Status":"Inprogress","Road_Name":null},"geometry":{"type":"MultiLineString","coordinates":[[[76.3,10.0,0]]]}},
 {"type":"Feature","properties":{"Proj_Code":"PWD015-102-02","Proj_Name":"182 Roads","District":"Ernakulam","LAC":"Aluva","FS_Amount":9.5,"Contr_Amnt":9.1,"Status":"Inprogress","Road_Name":"Over bridge"},"geometry":{"type":"MultiLineString","coordinates":[[[76.4,10.1,0]]]}},
 {"type":"Feature","properties":{"Proj_Code":"PWD015-102-02","Proj_Name":"182 Roads","District":"Ernakulam","LAC":"Aluva","FS_Amount":4.0,"Status":"Approved","Road_Name":"Link road"},"geometry":{"type":"MultiLineString","coordinates":[[[76.5,10.2,0]]]}}
]};
</script>"#;

    #[test]
    fn an_estimate_repeated_on_sibling_packages_is_counted_once_per_package_statewide() {
        let parsed = parse(SMALL_PAGE.as_bytes(), "Ernakulam").unwrap();
        let park = &parsed.projects[0];
        assert_eq!(park.code, "AGR001-01-01");
        assert_eq!(park.sub_project_code.as_deref(), Some("AGR001-01"));
        assert_eq!(park.estimate_shared_by, 2, "the Thrissur package counts, and its second pin does not");
        assert!(park.estimate_is_shared());
        assert_eq!(park.expenditure, Some(100));
        assert_eq!(park.group_expenditure, Some(150));

        let school = &parsed.projects[1];
        assert_eq!(school.estimate_shared_by, 1);
        assert!(!school.estimate_is_shared());
        assert_eq!(school.group_expenditure, None);
    }

    #[test]
    fn line_segments_of_one_contract_are_one_work() {
        let parsed = parse(SMALL_PAGE.as_bytes(), "Ernakulam").unwrap();
        let roads = parsed.projects.iter().find(|p| p.code == "PWD015-102-02").unwrap();
        let names: Vec<_> = roads.works.iter().map(|w| w.road_name.as_deref()).collect();
        assert_eq!(names, [Some("Over bridge"), Some("Link road")]);
        assert_eq!(roads.works_amount(), Some(135_000_000));
        assert_eq!(roads.estimate_shared_by, 0, "no estimate is published for works-only projects");
    }

    #[test]
    fn lenient_fields() {
        let m: MarkerRaw = serde_json::from_str(
            r#"{"lat":"10.1","lng":76,"d":"Ernakulam","c":"Aluva","s":null,
                "proj":{"Project Name":"  X  ","Estimated Amount (Cr)":"1,000","Proj Code":12},"link":[]}"#,
        )
        .unwrap();
        assert_eq!(m.lat, Some(10.1));
        assert_eq!(m.proj.name.as_deref(), Some("X"));
        assert_eq!(m.proj.estimated_amount, Some(1000.0));
        assert_eq!(m.proj.expenditure, None);
        assert_eq!(m.proj.code.as_deref(), Some("12"));
        assert!(m.link.is_none());
    }

    #[test]
    fn midpoint_of_first_line() {
        let g: Geometry = serde_json::from_str(
            r#"{"type":"MultiLineString","coordinates":[[[76.1,10.1,0],[76.2,10.2,0],[76.3,10.3,0]],[[1,2,3]]]}"#,
        )
        .unwrap();
        assert_eq!(g.coordinates.0, Some((76.2, 10.2)));
        let empty: Geometry = serde_json::from_str(r#"{"type":"MultiLineString","coordinates":[]}"#).unwrap();
        assert_eq!(empty.coordinates.0, None);
    }
}
