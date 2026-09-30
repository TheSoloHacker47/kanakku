//! Server-rendered pages. Templates are maud macros, compiled to plain string writes.

use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::inr_short;
use kanakku_core::i18n::{flag_label, Lang};
use kanakku_core::model::{headline, Headline};
use kanakku_core::names::{constituency_label, department_label};
use worker::url::form_urlencoded::byte_serialize;
use kanakku_core::stage::Stage;
use kanakku_core::title::display_title;
use kanakku_core::Date;
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::db::{ListRow, Point};
use crate::http::{encode_segment, CSS};
use crate::{district, icons};

pub mod entities;
pub mod funding;
pub mod home;
pub mod liability;
pub mod list;
pub mod pages;
pub mod project;

#[derive(Clone, Copy, PartialEq)]
pub enum Nav {
    Home,
    Projects,
    Funding,
    Map,
    Methodology,
    Data,
    None,
}

pub struct Page<'a> {
    pub lang: Lang,
    pub title: &'a str,
    pub description: &'a str,
    /// Path and query of this page without the language prefix, e.g. `/p/PWD016-05-01`.
    pub path: &'a str,
    /// Scheme and host of the request, for absolute links in share tags.
    pub origin: &'a str,
    pub nav: Nav,
    /// When the source was last checked, as a UTC timestamp.
    pub last_checked: Option<&'a str>,
    /// Extra `<head>` markup, used by the map page for its scripts.
    pub head: Markup,
}

pub fn layout(page: &Page, body: Markup) -> String {
    let lang = page.lang;
    let t = lang.t();
    let other = lang.other();
    let href = |path: &str| format!("{}{path}", lang.prefix());
    let link = |nav: Nav, path: &str, label: &str| {
        html! { a href=(href(path)) aria-current=[(page.nav == nav).then_some("page")] { (label) } }
    };
    let checked = page.last_checked.and_then(Date::from_utc_timestamp_ist).map(Date::to_dmy);
    let title = if page.nav == Nav::Home { format!("{} · {}", t.site_name, t.tagline) } else { format!("{} · {}", page.title, t.site_name) };
    let url = format!("{}{}{}", page.origin, lang.prefix(), page.path);

    html! {
        (DOCTYPE)
        html lang=(lang.code()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                meta name="description" content=(page.description);
                meta name="theme-color" content="#44d991";
                link rel="icon" href="/favicon.svg" type="image/svg+xml";
                link rel="apple-touch-icon" href="/icon-192.png";
                link rel="manifest" href="/manifest.webmanifest";
                link rel="canonical" href=(url);
                link rel="alternate" hreflang=(other.code()) href=(format!("{}{}{}", page.origin, other.prefix(), page.path));
                meta property="og:type" content="website";
                meta property="og:site_name" content=(t.site_name);
                meta property="og:title" content=(page.title);
                meta property="og:description" content=(page.description);
                meta property="og:url" content=(url);
                meta property="og:image" content=(format!("{}/og.png", page.origin));
                meta name="twitter:card" content="summary_large_image";
                style { (PreEscaped(CSS)) }
                (page.head)
            }
            body {
                a.skip href="#main" { (t.skip_to_content) }
                header.top {
                    div.wrap {
                        a.brand href=(href("/")) { (logo()) (t.site_name) }
                        nav aria-label=(lang.pick("പ്രധാന മെനു", "Main")) {
                            (link(Nav::Projects, "/projects", t.nav_projects))
                            (link(Nav::Funding, "/funding", lang.pick("പണം", "Money")))
                            (link(Nav::Map, "/map", t.nav_map))
                            (link(Nav::Methodology, "/methodology", t.nav_methodology))
                            (link(Nav::Data, "/data", t.nav_data))
                        }
                        a.btn.ghost.lang href=(format!("{}{}", other.prefix(), page.path)) lang=(other.code()) hreflang=(other.code()) {
                            (icon(icons::LANGUAGES)) (other.t().lang_name)
                        }
                    }
                }
                main #main { (body) }
                footer.foot {
                    div.wrap {
                        div {
                            a.brand href=(href("/")) { (logo()) (t.site_name) }
                            p { (t.footer_principle) }
                            p { (t.footer_corrections) }
                        }
                        div {
                            h2 { (lang.pick("താളുകൾ", "Pages")) }
                            ul {
                                li { a href=(href("/projects")) { (t.nav_projects) } }
                                li { a href=(href("/projects?flag=any")) { (lang.pick("സൂചനയുള്ള പദ്ധതികൾ", "Flagged projects")) } }
                                li { a href=(href("/funding")) { (funding::title(lang)) } }
                                li { a href=(href("/liability")) { (liability::title(lang)) } }
                                li { a href=(href("/contractors")) { (lang.pick("കരാറുകാർ", "Contractors")) } }
                                li { a href=(href("/agencies")) { (lang.pick("നിർവഹണ സ്ഥാപനങ്ങൾ", "Implementing agencies")) } }
                                li { a href=(href("/map")) { (t.nav_map) } }
                                li { a href=(href("/methodology")) { (t.nav_methodology) } }
                                li { a href=(href("/data")) { (t.nav_data) } }
                                li { a href=(href("/status")) { (lang.pick("പ്രവർത്തന നില", "Status")) } }
                            }
                        }
                        div {
                            h2 { (lang.pick("ഡാറ്റ", "Data")) }
                            ul {
                                li { a href="https://gis.kiifb.org/" rel="noopener" { (t.source_kiifb) } }
                                li { a href="https://www.kiifb.org/prjStatus.jsp" rel="noopener" { (lang.pick("കിഫ്ബി പ്രോജക്ട് സ്റ്റാറ്റസ്", "KIIFB project status")) } }
                                li {
                                    (lang.pick("അവസാനം പരിശോധിച്ചത്", "Last checked")) ": "
                                    @match &checked { Some(date) => (date), None => "—" }
                                }
                                li { a href="https://www.pwd.kerala.gov.in/IMF_website/Projects/wings_list.php" rel="noopener" { (lang.pick("പി.ഡബ്ല്യു.ഡി ഡി.എൽ.പി പട്ടിക", "PWD liability list")) } }
                                li { (lang.pick("ഭൂപടം", "Maps")) ": © OpenStreetMap" }
                            }
                        }
                        p.fine {
                            (lang.pick(
                                "കണക്ക് ഒരു സ്വതന്ത്ര പൗര സംരംഭമാണ്. കിഫ്ബിയുമായോ സർക്കാരുമായോ ബന്ധമില്ല.",
                                "Kanakku is an independent civic project. It is not affiliated with KIIFB or the government.",
                            ))
                        }
                    }
                }
            }
        }
    }
    .into_string()
}

/// The mark: a tally of five, the oldest way of keeping count.
pub fn logo() -> Markup {
    PreEscaped(
        r##"<svg viewBox="0 0 32 32" aria-hidden="true"><rect width="32" height="32" rx="7" fill="#44d991"/><path d="M9 8v16M13.7 8v16M18.3 8v16M23 8v16M5.5 20.5l21-9" fill="none" stroke="#0a0a0a" stroke-width="2.3" stroke-linecap="round"/></svg>"##
            .to_string(),
    )
}

/// A 24px stroke icon from the embedded set.
pub fn icon(inner: &str) -> Markup {
    html! {
        svg.i viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
            (PreEscaped(inner))
        }
    }
}

/// Text that stays in English on a Malayalam page (KIIFB publishes names and statuses in English).
pub fn en(text: &str) -> Markup {
    html! { span lang="en" { (text) } }
}

pub fn department_icon(name: Option<&str>) -> &'static str {
    let name = name.unwrap_or_default().to_lowercase();
    let has = |needle: &str| name.contains(needle);
    if has("higher education") {
        icons::GRADUATION_CAP
    } else if has("education") {
        icons::SCHOOL
    } else if has("public works") {
        icons::ROUTE
    } else if has("health") {
        icons::HEART_PULSE
    } else if has("water") {
        icons::DROPLETS
    } else if has("information technology") {
        icons::CPU
    } else if has("shipping") || has("navigation") {
        icons::SHIP
    } else if has("fisheries") {
        icons::FISH
    } else if has("sc&st") || has("scheduled") {
        icons::USERS
    } else if has("local self") {
        icons::BUILDING_2
    } else if has("registration") {
        icons::STAMP
    } else if has("power") || has("electric") {
        icons::ZAP
    } else if has("sport") {
        icons::TROPHY
    } else if has("agri") {
        icons::SPROUT
    } else if has("forest") {
        icons::TREES
    } else if has("transport") {
        icons::BUS
    } else if has("industr") {
        icons::FACTORY
    } else if has("tourism") {
        icons::PALMTREE
    } else if has("home") {
        icons::SHIELD
    } else if has("cultur") {
        icons::DRAMA
    } else {
        icons::LANDMARK
    }
}

/// Says what a row's figure is, because it differs between rows.
pub fn headline_label(lang: Lang, kind: Headline) -> &'static str {
    match kind {
        Headline::Estimate => lang.pick("കണക്കാക്കിയ തുക", "estimate"),
        Headline::Spent => lang.pick("ഇതുവരെ ചെലവ്", "spent so far"),
        Headline::WorksTotal => lang.pick("പ്രവൃത്തികളുടെ ആകെ", "total of works"),
    }
}

/// An amount with its unit set smaller: "₹21.43" + "crore".
pub fn big_amount(amount: i64, lang: Lang) -> Markup {
    let text = inr_short(amount, lang);
    match text.split_once(' ') {
        Some((figure, unit)) => html! { (figure) " " small { (unit) } },
        None => html! { (text) },
    }
}

/// One project as a list row. The whole row is the link.
pub fn project_row(lang: Lang, row: &ListRow) -> Markup {
    let figure = headline(row.estimated_amount, row.estimate_shared_by, row.expenditure, row.works_amount);
    let stage = row.stage.as_deref().and_then(Stage::parse);
    let constituencies: Vec<&str> = row.constituencies.as_deref().map(|c| c.split('|').collect()).unwrap_or_default();
    html! {
        li {
            a.row href=(format!("{}/p/{}", lang.prefix(), encode_segment(&row.code))) {
                span.ico { (icon(department_icon(row.department.as_deref()))) }
                span.t lang="en" { (display_title(&row.title_en)) }
                span.m {
                    @if let Some(department) = &row.department { (department_label(lang, department)) " · " }
                    @for (i, name) in constituencies.iter().enumerate() {
                        @if i > 0 { ", " }
                        (constituency_label(lang, name))
                    }
                    @if constituencies.is_empty() { span.code { (row.code) } }
                }
                span.amt {
                    @match figure {
                        Some((amount, kind)) => { b { (inr_short(amount, lang)) } span { (headline_label(lang, kind)) } },
                        None => span { (lang.pick("തുക പ്രസിദ്ധീകരിച്ചിട്ടില്ല", "No amount published")) },
                    }
                }
                @if row.flag_types.is_some() || stage.is_some() {
                    span.tags {
                        @if let Some(types) = &row.flag_types {
                            @for kind in types.split(',').filter_map(FlagKind::parse) {
                                span.badge.flag { (icon(icons::FLAG)) (flag_label(lang, kind)) }
                            }
                        }
                        @if let Some(stage) = stage { span.badge { (stage.label(lang)) } }
                    }
                }
            }
        }
    }
}

/// A small map with one dot per reported project location: the whole state with its district
/// borders when `scope` is empty, otherwise that one district.
/// `here` marks locations to highlight instead of the flagged ones.
pub fn dot_map(scope: &str, points: &[Point], here: &[(f64, f64)], label: &str, plain: bool) -> Markup {
    let one = district::get(scope);
    let frame = one.map(|d| &d.frame).unwrap_or(&district::STATE);
    // Thousands of dots are drawn as two paths of zero-length strokes, not as thousands of elements.
    let (mut normal, mut flagged) = (String::new(), String::new());
    let mut drawn = std::collections::HashSet::new();
    for p in points {
        let Some((x, y)) = frame.project(p.lat, p.lng) else { continue };
        let is_flagged = p.flagged > 0 && here.is_empty();
        // Locations that land on the same spot are drawn once.
        if !drawn.insert((x.round() as i32, y.round() as i32, is_flagged)) {
            continue;
        }
        let path = if is_flagged { &mut flagged } else { &mut normal };
        path.push_str(&format!("M{x:.0} {y:.0}h0"));
    }
    let mut marks = String::new();
    for (lat, lng) in here {
        if let Some((x, y)) = frame.project(*lat, *lng) {
            marks.push_str(&format!(r#"<circle class="here" cx="{x:.1}" cy="{y:.1}" r="6"/>"#));
        }
    }
    html! {
        svg.dots.plain[plain].state[one.is_none()] viewBox=(format!("0 0 {} {}", frame.width, frame.height)) role="img" aria-label=(label) {
            @match one {
                Some(d) => path.land d=(d.outline) {},
                None => { @for d in &district::DISTRICTS { path.land d=(d.in_state) {} } },
            }
            @if !normal.is_empty() { path.pts d=(normal) {} }
            @if !flagged.is_empty() { path.pts.f d=(flagged) {} }
            (PreEscaped(marks))
        }
    }
}

/// Whether a location falls inside the box `dot_map` draws for a scope.
pub fn on_dot_map(scope: &str, lat: f64, lng: f64) -> bool {
    district::get(scope).map(|d| &d.frame).unwrap_or(&district::STATE).project(lat, lng).is_some()
}

/// `key=value` for a query string.
pub fn pair(key: &str, value: &str) -> String {
    format!("{key}={}", byte_serialize(value.as_bytes()).collect::<String>())
}

/// Previous and next links for a paged list. `href` builds the address of a page.
pub fn pager(lang: Lang, page: u32, pages: u32, href: &dyn Fn(u32) -> String) -> Markup {
    let t = lang.t();
    html! {
        @if pages > 1 {
            nav.pager aria-label=(t.page) {
                @if page > 1 {
                    a.btn.ghost.sm rel="prev" href=(href(page - 1)) { (icon(icons::ARROW_LEFT)) (t.previous) }
                } @else { span {} }
                span.small.muted { (t.page) " " (page) " / " (pages) }
                @if page < pages {
                    a.btn.ghost.sm rel="next" href=(href(page + 1)) { (t.next) (icon(icons::ARROW_RIGHT)) }
                } @else { span {} }
            }
        }
    }
}

/// A thin bar filled to `fraction` (0..=1). Drawn as SVG so no inline styles are needed.
pub fn meter(fraction: f64, class: &str) -> Markup {
    let width = (fraction.clamp(0.0, 1.0) * 100.0).max(if fraction > 0.0 { 1.5 } else { 0.0 });
    html! {
        svg.meter viewBox="0 0 100 8" preserveAspectRatio="none" aria-hidden="true" {
            rect class=(class) width=(format!("{width:.1}")) height="8" {}
        }
    }
}
