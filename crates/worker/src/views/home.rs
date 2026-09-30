//! The front page, for the whole state or for one district: what is here, what stands out, and ways in.

use kanakku_core::fmt::inr_short;
use kanakku_core::i18n::Lang;
use kanakku_core::names::{constituency_label, department_label, district_label};
use kanakku_core::stage::Stage;
use kanakku_core::title::display_title;
use kanakku_core::Date;
use maud::{html, Markup, PreEscaped};

use super::{big_amount, dash, department_icon, dot_map, en, funding, icon, layout, liability, meter, on_dot_map, pair, project_row, Nav, Page};
use crate::db::Home;
use crate::http::encode_segment;
use crate::icons;

/// `district` is empty for the state's page.
pub fn render(lang: Lang, origin: &str, data: &Home, district: &str) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let totals = &data.totals;
    let state = district.is_empty();
    let place = if state { lang.pick("കേരളം", "Kerala").to_string() } else { district_label(lang, district).to_string() };
    let checked = totals.last_checked.as_deref().and_then(Date::from_utc_timestamp_ist);
    let outside = data.points.iter().filter(|pt| !on_dot_map(district, pt.lat, pt.lng)).count();
    let top_department = data.departments.iter().map(|d| d.n).max().unwrap_or(1).max(1);
    // Every link into the lists keeps the district.
    let scoped = |path: &str, extra: &[String]| {
        let mut parts: Vec<String> = extra.to_vec();
        if !state {
            parts.push(pair("district", district));
        }
        if parts.is_empty() {
            format!("{p}{path}")
        } else {
            format!("{p}{path}?{}", parts.join("&"))
        }
    };
    let path = if state { "/".to_string() } else { format!("/d/{}", district.to_lowercase()) };

    let body = html! {
        section.hero {
            div.wrap {
                div {
                    p.eyebrow {
                        @if state {
                            (lang.pick("കേരളം · കിഫ്ബി പദ്ധതികൾ", "Kerala · KIIFB projects"))
                        } @else {
                            a href=(format!("{p}/")) { (lang.pick("കേരളം", "Kerala")) } " · "
                            @match lang {
                                Lang::Ml => { (place) " ജില്ല" },
                                Lang::En => { (place) " district" },
                            }
                        }
                    }
                    h1.d1 { (lang.pick("പൊതുപണം എവിടെ പോകുന്നു?", "Where the public money goes")) }
                    p.lead {
                        (lang.pick(
                            "അനുവദിച്ച തുക, കരാറുകാരൻ, നൽകിയ തുക, പുരോഗതി. ഓരോ സംഖ്യയും അതിന്റെ ഉറവിടത്തോടൊപ്പം.",
                            "What was sanctioned, who got the contract, what has been paid and how far the work has come. Every number carries its source.",
                        ))
                    }
                    form.find method="get" action=(format!("{p}/projects")) role="search" {
                        div.field {
                            (icon(icons::SEARCH))
                            label.vh for="q" { (t.search_label) }
                            input #q type="search" name="q" placeholder=(t.search_placeholder) autocomplete="off" enterkeyhint="search";
                        }
                        @if !state { input type="hidden" name="district" value=(district); }
                        button.btn type="submit" { (t.search_label) }
                    }
                }
                figure.tall[state] {
                    a href=(format!("{p}/map")) aria-label=(t.nav_map) {
                        (dot_map(district, &data.points, &[], &match lang {
                            Lang::Ml => format!("പദ്ധതി സ്ഥാനങ്ങളുള്ള {place} ഭൂപടം"),
                            Lang::En => format!("Map of {place} with project locations"),
                        }, false))
                    }
                    figcaption {
                        span {
                            (lang.pick("ഓരോ കുത്തും കിഫ്ബി രേഖപ്പെടുത്തിയ ഒരു പദ്ധതി സ്ഥാനം.", "Each dot is a project location as KIIFB records it."))
                            @if outside > 0 {
                                " "
                                @match lang {
                                    Lang::Ml => { (outside) " എണ്ണം ഈ ഭൂപടത്തിന് പുറത്താണ് രേഖപ്പെടുത്തിയിരിക്കുന്നത്." },
                                    Lang::En => { (outside) " are recorded outside this map." },
                                }
                            }
                        }
                        a.more href=(format!("{p}/map")) { (lang.pick("ഭൂപടം തുറക്കുക", "Open the map")) (icon(icons::ARROW_RIGHT)) }
                    }
                }
            }
        }

        div.wrap {
            div.stats {
                a.stat.k href=(scoped("/projects", &[])) {
                    span.num { (totals.total) }
                    p {
                        (lang.pick("പദ്ധതികൾ", "projects"))
                        span {
                            @match (lang, state) {
                                (Lang::Ml, true) => { (data.departments.len()) " വകുപ്പുകൾ, 14 ജില്ലകൾ" },
                                (Lang::En, true) => { (data.departments.len()) " departments, 14 districts" },
                                (Lang::Ml, false) => { (data.departments.len()) " വകുപ്പുകൾ, " (data.constituencies.len()) " മണ്ഡലങ്ങൾ" },
                                (Lang::En, false) => { (data.departments.len()) " departments, " (data.constituencies.len()) " constituencies" },
                            }
                        }
                    }
                }
                a.stat.b href=(scoped("/projects", &["sort=spent".into()])) {
                    span.num { @match totals.spent { Some(spent) => (big_amount(spent, lang)), None => (dash(lang)) } }
                    p {
                        (lang.pick("ഇതുവരെ ചെലവ്", "spent so far"))
                        span { (lang.pick("കിഫ്ബി രേഖപ്പെടുത്തിയത്", "as KIIFB reports it")) }
                    }
                }
                a.stat.f href=(scoped("/projects", &["flag=any".into()])) {
                    span.num { (totals.flagged) }
                    p {
                        (lang.pick("സൂചനയുള്ള പദ്ധതികൾ", "projects with a flag"))
                        span { (lang.pick("സൂചന ആരോപണമല്ല", "a flag is not an accusation")) }
                    }
                }
                a.stat href=(format!("{p}/status")) {
                    span.num {
                        @match checked {
                            Some(date) => { @let (_, m, d) = date.ymd(); (d) " " small { (month(lang, m)) } },
                            None => (dash(lang)),
                        }
                    }
                    p {
                        (lang.pick("അവസാനം പരിശോധിച്ചത്", "last checked"))
                        span { (lang.pick("ദിവസവും കിഫ്ബി ഡാഷ്ബോർഡുമായി", "against the KIIFB dashboard, daily")) }
                    }
                }
            }

            @if state && !data.districts.is_empty() {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (lang.pick("ജില്ല തിരിച്ച്", "By district")) }
                            p { (lang.pick(
                                "ഒരു ജില്ല തിരഞ്ഞെടുത്താൽ അവിടത്തെ പദ്ധതികളും മണ്ഡലങ്ങളും കാണാം.",
                                "Pick a district to see its projects and constituencies.",
                            )) }
                        }
                    }
                    div.tiles {
                        @for d in &data.districts {
                            @let href = if d.v == "-" { format!("{p}/projects?district=-") } else { format!("{p}/d/{}", d.v.to_lowercase()) };
                            a.tile href=(href) {
                                b { (district_label(lang, if d.v == "-" { "" } else { &d.v })) }
                                @if let Some(spent) = d.spent.filter(|s| *s > 0) {
                                    span.small.muted { (inr_short(spent, lang)) " " (lang.pick("ചെലവ്", "spent")) }
                                }
                                span.n {
                                    (d.n) small { (lang.pick("പദ്ധതികൾ", "projects")) }
                                    @if let Some(flagged) = d.flagged.filter(|n| *n > 0) { " " span.badge.flag { (icon(icons::FLAG)) (flagged) span.vh { " " (lang.pick("സൂചനയുള്ളവ", "with a flag")) } } }
                                }
                            }
                        }
                    }
                }
            }

            @if data.funding.total > 0 {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (funding::title(lang)) }
                            p {
                                @match lang {
                                    Lang::Ml => { "കിഫ്ബിയുടെ സ്വന്തം പ്രോജക്ട് സ്റ്റാറ്റസ് താളിൽ നിന്ന്: " (place) " എന്നതിന് കീഴിലുള്ള പദ്ധതികൾക്ക് അനുവദിച്ച തുകയും ഇതുവരെ നൽകിയ തുകയും." },
                                    Lang::En => { "From KIIFB's own project status page: what it approved for the projects listed under " (place) ", and what has been paid." },
                                }
                                @if !state { " " (lang.pick("ചിലത് പല ജില്ലകളിലായുള്ളവയാണ്.", "Some span several districts.")) }
                            }
                        }
                        a.more href=(scoped("/funding", &[])) { (lang.pick("ഓരോ പദ്ധതിയും", "Project by project")) (icon(icons::ARROW_RIGHT)) }
                    }
                    (funding::totals_panels(lang, &data.funding, Some(&scoped("/funding", &[]))))
                }
            }

            @if data.liability > 0 {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (liability::title(lang)) }
                            p {
                                @match lang {
                                    Lang::Ml => { "പൂർത്തിയായ " b { (data.liability) } " പൊതുമരാമത്ത് പ്രവൃത്തികളിൽ തകരാർ വന്നാൽ പരിഹരിക്കാൻ കരാറുകാരന് ഇപ്പോഴും ബാധ്യതയുണ്ട്. ഏത് റോഡ്, ഏത് കെട്ടിടം, ഏത് കരാറുകാരൻ, എന്നു വരെ." },
                                    Lang::En => { "For " b { (data.liability) } " finished PWD works, the contractor is still liable to repair defects. Which road or building, which contractor, and until when." },
                                }
                            }
                        }
                        a.more href=(scoped("/liability", &[])) { (lang.pick("പട്ടിക കാണുക", "See the list")) (icon(icons::ARROW_RIGHT)) }
                    }
                }
            }

            @if !data.flagged.is_empty() {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (lang.pick("സൂചനയുള്ള പദ്ധതികൾ", "Flagged right now")) }
                            p { (lang.pick(
                                "പ്രസിദ്ധീകരിച്ച സംഖ്യകൾ ഒരു പരിധി കടന്ന പദ്ധതികൾ. സൂചന തെറ്റ് നടന്നതിന്റെ തെളിവല്ല.",
                                "Projects whose published figures crossed a threshold. A flag is not evidence of wrongdoing.",
                            )) }
                        }
                        a.more href=(format!("{p}/methodology#rules")) { (lang.pick("സൂചനകൾ എന്താണ്?", "What the flags mean")) (icon(icons::ARROW_RIGHT)) }
                    }
                    ol.rows { @for row in &data.flagged { (project_row(lang, row)) } }
                    p.actions { a.btn.ghost.sm href=(scoped("/projects", &["flag=any".into()])) { (lang.pick("സൂചനയുള്ള എല്ലാ പദ്ധതികളും", "All flagged projects")) (icon(icons::ARROW_RIGHT)) } }
                }
            }

            section.sec {
                div.sec-h {
                    h2.d2 { (lang.pick("വകുപ്പ് തിരിച്ച്", "By department")) }
                    a.more href=(scoped("/projects", &[])) { (lang.pick("എല്ലാ പദ്ധതികളും", "All projects")) (icon(icons::ARROW_RIGHT)) }
                }
                ul.bars {
                    @for d in &data.departments {
                        li {
                            a href=(scoped("/projects", &[pair("dept", &d.v)])) {
                                span.ico { (icon(department_icon(Some(&d.v)))) }
                                span {
                                    span.name { (department_label(lang, &d.v)) }
                                    (meter(d.n as f64 / top_department as f64, ""))
                                }
                                span.n {
                                    (d.n) span.vh { " " (lang.pick("പദ്ധതികൾ", "projects")) }
                                    small {
                                        @match d.spent.filter(|s| *s > 0) {
                                            Some(spent) => { (inr_short(spent, lang)) span.vh { " " (lang.pick("ചെലവ്", "spent")) } },
                                            None => (dash(lang)),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            @if !state && !data.constituencies.is_empty() {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (lang.pick("മണ്ഡലം തിരിച്ച്", "By constituency")) }
                            p { (lang.pick(
                                "എം.എൽ.എമാരുടെ പേരുകൾ കിഫ്ബി ഡാഷ്ബോർഡിൽ ഉള്ളതുപോലെ.",
                                "MLA names are as the KIIFB dashboard lists them.",
                            )) }
                        }
                    }
                    div.tiles {
                        @for c in &data.constituencies {
                            a.tile href=(scoped("/projects", &[pair("lac", &c.v)])) {
                                b { (constituency_label(lang, &c.v)) }
                                @let mla = match lang { Lang::Ml => c.mla_ml.as_deref().or(c.mla.as_deref()), Lang::En => c.mla.as_deref() };
                                @if let Some(mla) = mla { span.small.muted { (mla) } }
                                span.n {
                                    (c.n) small { (lang.pick("പദ്ധതികൾ", "projects")) }
                                    @if let Some(flagged) = c.flagged.filter(|n| *n > 0) { " " span.badge.flag { (icon(icons::FLAG)) (flagged) span.vh { " " (lang.pick("സൂചനയുള്ളവ", "with a flag")) } } }
                                }
                            }
                        }
                    }
                }
            }

            (stages(lang, data, &scoped))

            @if !data.largest.is_empty() {
                section.sec {
                    div.sec-h {
                        h2.d2 { (lang.pick("ഏറ്റവും കൂടുതൽ ചെലവായവ", "Most spent so far")) }
                        a.more href=(scoped("/projects", &["sort=spent".into()])) { (lang.pick("മുഴുവൻ പട്ടിക", "Full list")) (icon(icons::ARROW_RIGHT)) }
                    }
                    ol.rows { @for row in &data.largest { (project_row(lang, row)) } }
                }
            }

            @if !data.changes.is_empty() {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (lang.pick("പുതിയ മാറ്റങ്ങൾ", "Latest changes")) }
                            p { (lang.pick("ഡാഷ്ബോർഡിൽ മാറിയതായി ഞങ്ങൾ രേഖപ്പെടുത്തിയ വിവരങ്ങൾ.", "Figures we recorded changing on the dashboard.")) }
                        }
                    }
                    ul.list {
                        @for c in &data.changes {
                            li {
                                a href=(format!("{p}/p/{}", encode_segment(&c.code))) lang="en" { b { (display_title(&c.title_en)) } }
                                " " span.small.muted {
                                    (Date::parse_iso(&c.observed_on).map(Date::to_dmy).unwrap_or_default()) " · "
                                    span.code { (c.field) } ": "
                                    (en(c.old_value.as_deref().unwrap_or("—"))) " → " (en(c.new_value.as_deref().unwrap_or("—")))
                                }
                            }
                        }
                    }
                }
            }

            section.sec {
                div.sec-h { h2.d2 { (lang.pick("ഇത് എങ്ങനെ വായിക്കാം", "How to read this")) } }
                div.how {
                    div {
                        (icon(icons::DATABASE))
                        h3 { (lang.pick("ഉറവിടം നിങ്ങൾക്ക് കാണാം", "You can see the source")) }
                        p { (lang.pick(
                            "എല്ലാ സംഖ്യകളും സർക്കാർ സ്ഥാപനങ്ങളുടെ സ്വന്തം താളുകളിൽ നിന്നാണ്. ആ താളിന്റെ തീയതിയുള്ള പകർപ്പ് ഞങ്ങൾ സൂക്ഷിക്കുന്നു; അത് നിങ്ങൾക്ക് ഡൗൺലോഡ് ചെയ്യാം.",
                            "Every figure comes from a government body's own page. We keep a dated copy of that page, and you can download it.",
                        )) }
                    }
                    div {
                        (icon(icons::SCALE))
                        h3 { (lang.pick("എല്ലാവർക്കും ഒരേ നിയമം", "One rule for everyone")) }
                        p { (lang.pick(
                            "പ്രസിദ്ധീകരിച്ച പരിധികളിൽ നിന്നാണ് സൂചനകൾ. എല്ലാ പദ്ധതിക്കും കക്ഷിക്കും മണ്ഡലത്തിനും ഒരുപോലെ ബാധകം.",
                            "Flags come from published thresholds, applied the same way to every project, party and constituency.",
                        )) }
                    }
                    div {
                        (icon(icons::CIRCLE_HELP))
                        h3 { (lang.pick("അറിയാത്തതും പറയുന്നു", "We say what is missing")) }
                        p { (lang.pick(
                            "കരാറുകാരൻ, നൽകിയ തുക തുടങ്ങി ഉറവിടം പ്രസിദ്ധീകരിക്കാത്ത വിവരങ്ങൾ ഓരോ പദ്ധതിയുടെയും താളിൽ എടുത്തുപറയുന്നു.",
                            "Each project page lists what the source does not publish, such as the contractor or the payments.",
                        )) }
                    }
                }
            }
        }
    };

    let title = if state { t.site_name.to_string() } else { place.clone() };
    let description = if state {
        t.list_intro.to_string()
    } else {
        match lang {
            Lang::Ml => format!("{place} ജില്ലയിലെ {} കിഫ്ബി പദ്ധതികൾ. {}", totals.total, t.list_intro),
            Lang::En => format!("{} KIIFB projects in {place} district. {}", totals.total, t.list_intro),
        }
    };
    layout(
        &Page {
            lang,
            title: &title,
            description: &description,
            path: &path,
            origin,
            nav: if state { Nav::Home } else { Nav::None },
            last_checked: totals.last_checked.as_deref(),
            head: html! {},
            image: None,
        },
        body,
    )
}

/// One stacked bar: how many projects sit at each stage.
fn stages(lang: Lang, data: &Home, scoped: &dyn Fn(&str, &[String]) -> String) -> Markup {
    let counts: Vec<(Stage, u32)> = Stage::ALL
        .into_iter()
        .filter_map(|stage| data.stages.iter().find(|f| f.v == stage.as_str()).map(|f| (stage, f.n)))
        .collect();
    let total: u32 = counts.iter().map(|(_, n)| n).sum();
    if total == 0 {
        return html! {};
    }
    let mut x = 0.0;
    let mut rects = String::new();
    for (stage, n) in &counts {
        let width = *n as f64 / total as f64 * 100.0;
        rects.push_str(&format!(r#"<rect class="s{}" x="{x:.2}" width="{width:.2}" height="10"/>"#, *stage as usize));
        x += width;
    }
    html! {
        section.sec {
            div.sec-h {
                div {
                    h2.d2 { (lang.pick("പദ്ധതികൾ ഏത് ഘട്ടത്തിൽ", "Where projects stand")) }
                    p { (lang.pick(
                        "കിഫ്ബിയുടെ സ്ഥിതിവിവരങ്ങളെ ഞങ്ങൾ ലളിതമായ ഘട്ടങ്ങളാക്കി തിരിച്ചതാണ് ഇത്.",
                        "KIIFB's status labels, grouped by us into plain stages.",
                    )) }
                }
            }
            svg.stack viewBox="0 0 100 10" preserveAspectRatio="none" role="img" aria-label=(lang.pick("ഘട്ടം തിരിച്ചുള്ള പദ്ധതികളുടെ എണ്ണം", "Projects by stage")) {
                (PreEscaped(rects))
            }
            div.key {
                @for (stage, n) in &counts {
                    a href=(scoped("/projects", &[format!("stage={}", stage.as_str())])) {
                        svg viewBox="0 0 10 10" aria-hidden="true" { rect class=(format!("s{}", *stage as usize)) width="10" height="10" {} }
                        (stage.label(lang)) " " b { (n) }
                    }
                }
            }
        }
    }
}

pub fn month(lang: Lang, m: u32) -> &'static str {
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const ML: [&str; 12] = ["ജനു", "ഫെബ്രു", "മാർച്ച്", "ഏപ്രിൽ", "മേയ്", "ജൂൺ", "ജൂലൈ", "ഓഗ", "സെപ്റ്റം", "ഒക്ടോ", "നവം", "ഡിസം"];
    let i = (m.clamp(1, 12) - 1) as usize;
    match lang {
        Lang::Ml => ML[i],
        Lang::En => EN[i],
    }
}
