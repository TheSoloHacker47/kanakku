//! Contractors and implementing agencies: one page each, across the sources.

use kanakku_core::entity::{agency_display, contractor_display};
use kanakku_core::fmt::{inr, inr_short};
use kanakku_core::i18n::Lang;
use kanakku_core::title::display_title;
use kanakku_core::Date;
use maud::{html, Markup};

use super::{big_amount, dash, en, funding, icon, layout, liability, meter, pager, pair, project_row, Nav, Page};
use crate::db::{AgencyPage, AgencyRow, ContractorPage, ContractorRow, CONTRACTOR_PAGE_SIZE};
use crate::http::encode_segment;
use crate::icons;

fn head(lang: Lang, crumbs: &[(&str, Option<String>)], title: &str, lead: Markup) -> Markup {
    html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{}/", lang.prefix())) { (lang.t().site_name) }
                    @for (label, href) in crumbs {
                        (icon(icons::CHEVRON_RIGHT))
                        @match href { Some(href) => a href=(href) { (label) }, None => span { (label) } }
                    }
                }
                h1.d2 lang="en" { (title) }
                p.lead { (lead) }
            }
        }
    }
}

fn same_name_note(lang: Lang) -> &'static str {
    lang.pick(
        "ഒരേ പേരിന്റെ വ്യത്യസ്ത അക്ഷരവിന്യാസങ്ങൾ ഞങ്ങൾ ഒന്നായി കണക്കാക്കുന്നു. അതിനാൽ ഒരേ പേരുള്ള രണ്ട് വ്യത്യസ്ത കരാറുകാർ ഒന്നായി വന്നിരിക്കാം; ഉറവിടങ്ങൾ കരാറുകാർക്ക് തിരിച്ചറിയൽ നമ്പർ നൽകുന്നില്ല.",
        "We treat different spellings of a name as one contractor. Two different contractors with the same name may therefore appear as one; the sources give contractors no identifier.",
    )
}

fn contractor_name(row: &ContractorRow) -> String {
    contractor_display(row.pwd_name.as_deref().or(row.kiifb_name.as_deref()).unwrap_or(&row.key))
}

pub fn contractors_title(lang: Lang) -> &'static str {
    lang.pick("കരാറുകാർ", "Contractors")
}

pub fn contractors(lang: Lang, origin: &str, q: &str, page: u32, rows: &[ContractorRow], matching: u32) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let title = contractors_title(lang);
    let lead = lang.pick(
        "കിഫ്ബി ഡാഷ്ബോർഡിലും പി.ഡബ്ല്യു.ഡിയുടെ ബാധ്യതാ പട്ടികയിലും പേരുള്ള കരാറുകാർ, ഓരോരുത്തരുടെയും പ്രവൃത്തികളുടെ എണ്ണത്തോടെ.",
        "Contractors named on the KIIFB dashboard and on PWD's liability list, with the number of works each is named on.",
    );
    let pages = matching.div_ceil(CONTRACTOR_PAGE_SIZE).max(1);
    let href = |page: u32| {
        let mut parts = Vec::new();
        if !q.is_empty() {
            parts.push(pair("q", q));
        }
        if page > 1 {
            parts.push(format!("page={page}"));
        }
        if parts.is_empty() { format!("{p}/contractors") } else { format!("{p}/contractors?{}", parts.join("&")) }
    };
    let path = href(page).trim_start_matches(p).to_string();
    let body = html! {
        (head(lang, &[(title, None)], title, html! { (lead) }))
        div.wrap {
            form.find.sec method="get" action=(format!("{p}/contractors")) role="search" {
                div.field {
                    (icon(icons::SEARCH))
                    label.vh for="q" { (t.search_label) }
                    input #q type="search" name="q" value=(q) placeholder=(lang.pick("കരാറുകാരന്റെ പേര്", "Contractor's name")) autocomplete="off" enterkeyhint="search";
                }
                button.btn type="submit" { (t.search_label) }
            }
            p.note.sec { (same_name_note(lang)) }
            div.result {
                span.small.muted {
                    @match lang { Lang::Ml => { (matching) " കരാറുകാർ" }, Lang::En => { (matching) " contractors" } }
                }
            }
            @if rows.is_empty() {
                div.empty { p { b { (t.no_results) } } p.actions { a.btn.ghost href=(format!("{p}/contractors")) { (lang.pick("എല്ലാം കാണുക", "Show all")) } } }
            } @else {
                ol.rows {
                    @for row in rows {
                        li {
                            a.row href=(format!("{p}/c/{}", encode_segment(&row.key))) {
                                span.ico { (icon(icons::USERS)) }
                                span.t lang="en" { (contractor_name(row)) }
                                span.m {
                                    @if row.kiifb_works > 0 {
                                        @match lang { Lang::Ml => { "കിഫ്ബി പ്രവൃത്തികൾ: " (row.kiifb_works) }, Lang::En => { (row.kiifb_works) " on the KIIFB dashboard" } }
                                    }
                                    @if row.kiifb_works > 0 && row.pwd_works > 0 { " · " }
                                    @if row.pwd_works > 0 {
                                        @match lang { Lang::Ml => { "പി.ഡബ്ല്യു.ഡി ബാധ്യതാ പട്ടികയിൽ: " (row.pwd_works) }, Lang::En => { (row.pwd_works) " on PWD's liability list" } }
                                    }
                                }
                                @if let Some(value) = row.contract_value.filter(|v| *v > 0) {
                                    span.amt { b { (inr_short(value, lang)) } span { (lang.pick("കിഫ്ബി കരാർ തുക", "in KIIFB contracts")) } }
                                }
                            }
                        }
                    }
                }
                (pager(lang, page, pages, &href))
            }
        }
    };
    layout(&Page { lang, title, description: lead, path: &path, origin, nav: Nav::None, last_checked: None, head: html! {}, image: None }, body)
}

pub fn contractor(lang: Lang, origin: &str, key: &str, data: &ContractorPage, today: Date) -> String {
    let p = lang.prefix();
    let raw = data
        .liability
        .iter()
        .find_map(|w| w.contractor.as_deref())
        .or_else(|| data.works.first().map(|w| w.contractor_name.as_str()))
        .unwrap_or(key);
    let name = contractor_display(raw);
    // Every spelling the sources use, without the addresses KIIFB appends.
    let mut spellings: Vec<String> = data
        .works
        .iter()
        .map(|w| w.contractor_name.as_str())
        .chain(data.liability.iter().filter_map(|w| w.contractor.as_deref()))
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    spellings.sort();
    spellings.dedup();
    let contract_value: i64 = data.works.iter().filter_map(|w| w.contract_amount).sum();
    let paid: i64 = data.works.iter().filter_map(|w| w.paid_amount).sum();
    let active = data.liability.iter().filter(|w| w.ends_on.as_deref().and_then(Date::parse_iso).is_some_and(|end| end >= today)).count();
    let path = format!("/c/{}", encode_segment(key));
    let lead = match lang {
        Lang::Ml => format!("കിഫ്ബി ഡാഷ്ബോർഡിൽ {} പ്രവൃത്തികൾ, പി.ഡബ്ല്യു.ഡി ബാധ്യതാ പട്ടികയിൽ {} പ്രവൃത്തികൾ.", data.works.len(), data.liability.len()),
        Lang::En => format!("Named on {} on the KIIFB dashboard and {} on PWD's liability list.", works(data.works.len()), works(data.liability.len())),
    };

    let body = html! {
        (head(lang, &[(contractors_title(lang), Some(format!("{p}/contractors")))], &name, html! { (lead) }))
        div.wrap {
            div.panels {
                div.panel.g {
                    h2 { (lang.pick("കിഫ്ബി കരാർ തുക", "Value of KIIFB contracts")) }
                    @if contract_value > 0 {
                        p.num { (big_amount(contract_value, lang)) }
                        p.small { (inr(contract_value)) }
                    } @else {
                        p.num { (dash(lang)) }
                        p.small { (lang.pick("കിഫ്ബി ഡാഷ്ബോർഡിൽ കരാർ തുക ലഭ്യമല്ല.", "No contract amount on the KIIFB dashboard.")) }
                    }
                }
                div.panel {
                    h2 { (lang.pick("കിഫ്ബി നൽകിയതായി രേഖപ്പെടുത്തിയത്", "Recorded as paid by KIIFB")) }
                    @if paid > 0 {
                        p.num { (big_amount(paid, lang)) }
                        @if contract_value > 0 && paid <= contract_value { (meter(paid as f64 / contract_value as f64, "")) }
                    } @else {
                        p.num { (dash(lang)) }
                    }
                }
                div.panel {
                    h2 { (lang.pick("ബാധ്യതാ കാലാവധിയിലുള്ള പി.ഡബ്ല്യു.ഡി പ്രവൃത്തികൾ", "PWD works under liability now")) }
                    p.num { (active) }
                }
            }
            p.note.sec { (same_name_note(lang)) }

            @if !data.works.is_empty() {
                section.sec {
                    h2.h { (lang.pick("കിഫ്ബി ഡാഷ്ബോർഡിലെ പ്രവൃത്തികൾ", "Works on the KIIFB dashboard")) " (" (data.works.len()) ")" }
                    ol.rows {
                        @for w in &data.works {
                            li {
                                a.row href=(format!("{p}/p/{}", encode_segment(&w.code))) {
                                    span.ico { (icon(icons::ROUTE)) }
                                    span.t lang="en" { (display_title(&w.title_en)) }
                                    span.m {
                                        span.code { (w.code) }
                                        @if let Some(road) = &w.road_name { " · " (en(road)) }
                                    }
                                    span.amt {
                                        @match w.contract_amount {
                                            Some(amount) => { b { (inr_short(amount, lang)) } span { (lang.pick("കരാർ തുക", "contract amount")) } },
                                            None => span { (lang.pick("കരാർ തുക പ്രസിദ്ധീകരിച്ചിട്ടില്ല", "Contract amount not published")) },
                                        }
                                        @if let Some(paid) = w.paid_amount { span { " · " (inr_short(paid, lang)) " " (lang.pick("നൽകി", "paid")) } }
                                    }
                                    span.tags {
                                        @if w.flag_count > 0 { span.badge.flag { (icon(icons::FLAG)) (w.flag_count) span.vh { " " (lang.pick("സൂചനകൾ", "flags")) } } }
                                        @if let Some(status) = &w.status { span.badge lang="en" { (status) } }
                                        @if let Some(end) = w.scheduled_end.as_deref().and_then(Date::parse_iso) {
                                            span.badge { (lang.pick("നിശ്ചയിച്ച പൂർത്തീകരണം", "Scheduled end")) " " (end.to_dmy()) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            @if !data.liability.is_empty() {
                section.sec {
                    h2.h { (lang.pick("പി.ഡബ്ല്യു.ഡി ബാധ്യതാ പട്ടികയിലെ പ്രവൃത്തികൾ", "Works on PWD's liability list")) " (" (data.liability.len()) ")" }
                    ol.rows { @for row in &data.liability { (liability::work_row(lang, row, today)) } }
                }
            }

            @if spellings.len() > 1 {
                section.sec {
                    h2.h { (lang.pick("ഉറവിടങ്ങളിൽ പേര് എഴുതിയിരിക്കുന്ന രീതികൾ", "The name as the sources write it")) }
                    ul.list { @for s in &spellings { li lang="en" { (s) } } }
                }
            }
        }
    };
    layout(&Page { lang, title: &name, description: &lead, path: &path, origin, nav: Nav::None, last_checked: None, head: html! {}, image: None }, body)
}

fn works(n: usize) -> String {
    if n == 1 { "1 work".to_string() } else { format!("{n} works") }
}

pub fn agencies_title(lang: Lang) -> &'static str {
    lang.pick("നിർവഹണ സ്ഥാപനങ്ങൾ", "Implementing agencies")
}

pub fn agencies(lang: Lang, origin: &str, rows: &[AgencyRow]) -> String {
    let p = lang.prefix();
    let title = agencies_title(lang);
    let lead = lang.pick(
        "കിഫ്ബി പദ്ധതികൾ നടപ്പാക്കുന്ന സ്ഥാപനങ്ങൾ (SPV): ഓരോന്നിനും അനുവദിച്ച തുക, നൽകിയ തുക, പദ്ധതികളുടെ എണ്ണം.",
        "The bodies that carry out KIIFB projects (SPVs): what was approved for each, what has been paid, and how many projects.",
    );
    let body = html! {
        (head(lang, &[(title, None)], title, html! { (lead) }))
        div.wrap {
            @if rows.iter().any(|row| row.unread > 0) {
                p.note.sec { (lang.pick(
                    "ചില പദ്ധതികളുടെ പ്രവൃത്തിപ്പട്ടിക ഇതുവരെ വായിച്ചിട്ടില്ല. അവ ഉൾപ്പെടുന്ന സ്ഥാപനങ്ങളുടെ “നൽകിയ തുക” താൽക്കാലികമാണ്; യഥാർത്ഥത്തേക്കാൾ കൂടുതലായിരിക്കാം.",
                    "The work tables of some projects have not been read yet. For agencies with such projects the paid figure is provisional and may be overstated.",
                )) }
            }
            ol.rows.sec {
                @for row in rows {
                    li {
                        a.row href=(format!("{p}/a/{}", encode_segment(&row.key))) {
                            span.ico { (icon(icons::LANDMARK)) }
                            span.t lang="en" { (agency_display(&row.name)) }
                            span.m {
                                @match lang {
                                    Lang::Ml => { (row.funded) " പദ്ധതികൾ · ഭൂപടത്തിൽ " (row.packages) " പാക്കേജുകൾ" },
                                    Lang::En => { (row.funded) " projects · " (row.packages) " packages on the map" },
                                }
                            }
                            span.amt {
                                @match (row.approved.filter(|a| *a > 0), row.paid) {
                                    (Some(approved), paid) => {
                                        b { (inr_short(paid.unwrap_or(0), lang)) }
                                        span {
                                            @match lang {
                                                Lang::Ml => { "നൽകി · അനുവദിച്ചത് " (inr_short(approved, lang)) },
                                                Lang::En => { "paid of " (inr_short(approved, lang)) " approved" },
                                            }
                                        }
                                    },
                                    _ => span { (lang.pick("അനുവദിച്ച തുക ലഭ്യമല്ല", "No approved amount listed")) },
                                }
                            }
                            @if let (Some(approved), Some(paid)) = (row.approved.filter(|a| *a > 0), row.paid) {
                                span.bar { (meter(paid as f64 / approved as f64, "")) }
                            }
                            @if row.flagged > 0 || row.unread > 0 {
                                span.tags {
                                    @if row.flagged > 0 { span.badge.flag { (icon(icons::FLAG)) (row.flagged) span.vh { " " (lang.pick("സൂചനയുള്ളവ", "with a flag")) } } }
                                    @if row.unread > 0 { span.badge { (lang.pick("നൽകിയ തുക താൽക്കാലികം", "Paid figure provisional")) } }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    layout(&Page { lang, title, description: lead, path: "/agencies", origin, nav: Nav::None, last_checked: None, head: html! {}, image: None }, body)
}

pub fn agency(lang: Lang, origin: &str, data: &AgencyPage) -> String {
    let p = lang.prefix();
    let row = &data.row;
    let name = agency_display(&row.name);
    let path = format!("/a/{}", encode_segment(&row.key));
    let lead = match lang {
        Lang::Ml => format!("കിഫ്ബിയുടെ സ്റ്റാറ്റസ് താളിൽ {} പദ്ധതികൾ, ഭൂപട ഡാഷ്ബോർഡിൽ {} പാക്കേജുകൾ.", row.funded, row.packages),
        Lang::En => format!("{} projects on KIIFB's status page and {} packages on the map dashboard.", row.funded, row.packages),
    };
    let approved = row.approved.unwrap_or(0);
    let paid = row.paid.unwrap_or(0);
    let body = html! {
        (head(lang, &[(agencies_title(lang), Some(format!("{p}/agencies")))], &name, html! { (lead) }))
        div.wrap {
            div.panels {
                div.panel.g {
                    h2 { (lang.pick("കിഫ്ബി അനുവദിച്ചത്", "Approved by KIIFB")) }
                    @if approved > 0 { p.num { (big_amount(approved, lang)) } p.small { (inr(approved)) } } @else { p.num { (dash(lang)) } }
                }
                div.panel {
                    h2 { (lang.pick("ഇതുവരെ നൽകിയത്", "Paid so far")) }
                    @if row.funded > 0 {
                        p.num { (big_amount(paid, lang)) }
                        p.small { (inr(paid)) }
                        @if approved > 0 && paid <= approved { (meter(paid as f64 / approved as f64, "")) }
                    } @else { p.num { (dash(lang)) } }
                }
                div.panel {
                    h2 { (lang.pick("ഭൂപടത്തിലെ പാക്കേജുകൾ", "Packages on the map")) }
                    p.num { (row.packages) }
                    @if row.flagged > 0 {
                        p.small {
                            a href=(format!("{p}/projects?{}&flag=any", pair("agency", &row.key))) {
                                @match lang { Lang::Ml => { (row.flagged) " എണ്ണത്തിന് സൂചന" }, Lang::En => { (row.flagged) " flagged" } }
                            }
                        }
                    }
                }
            }

            @if !data.funded.is_empty() {
                section.sec {
                    h2.h { (funding::title(lang)) " (" (row.funded) ")" }
                    ol.rows { @for f in &data.funded { (funding::list_row(lang, f)) } }
                }
            }

            @if !data.packages.is_empty() {
                section.sec {
                    div.sec-h {
                        h2.d2 { (lang.pick("ഭൂപട ഡാഷ്ബോർഡിലെ പാക്കേജുകൾ", "Packages on the map dashboard")) }
                        a.more href=(format!("{p}/projects?{}", pair("agency", &row.key))) {
                            @match lang { Lang::Ml => { "എല്ലാ " (row.packages) " എണ്ണവും" }, Lang::En => { "All " (row.packages) } }
                            (icon(icons::ARROW_RIGHT))
                        }
                    }
                    ol.rows { @for r in &data.packages { (project_row(lang, r)) } }
                }
            }
        }
    };
    layout(&Page { lang, title: &name, description: &lead, path: &path, origin, nav: Nav::None, last_checked: None, head: html! {}, image: None }, body)
}
