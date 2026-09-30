//! Finished PWD works whose contractor is still liable for repairs.

use kanakku_core::entity::contractor_display;
use kanakku_core::fmt::{inr, inr_short};
use kanakku_core::i18n::Lang;
use kanakku_core::names::district_label;
use kanakku_core::pwd_dlp::{self as dlp, work_display};
use kanakku_core::Date;
use maud::{html, Markup};

use super::{en, icon, layout, pager, pair, Nav, Page};
use crate::db::{Liability, LiabilityFilter, LiabilityRow, LIABILITY_PAGE_SIZE};
use crate::icons;

/// A liability ending within this many days is marked as ending soon.
pub const SOON_DAYS: i32 = 90;

pub fn title(lang: Lang) -> &'static str {
    lang.pick("അറ്റകുറ്റപ്പണി ആരുടെ ബാധ്യത?", "Who must repair it")
}

fn href(lang: Lang, filter: &LiabilityFilter) -> String {
    let mut parts = Vec::new();
    if !filter.district.is_empty() {
        parts.push(pair("district", &filter.district));
    }
    if !filter.wing.is_empty() {
        parts.push(pair("wing", &filter.wing));
    }
    if !filter.contractor.is_empty() {
        parts.push(pair("c", &filter.contractor));
    }
    if filter.page > 1 {
        parts.push(format!("page={}", filter.page));
    }
    let query = if parts.is_empty() { String::new() } else { format!("?{}", parts.join("&")) };
    format!("{}/liability{query}", lang.prefix())
}

pub fn render(lang: Lang, origin: &str, filter: &LiabilityFilter, data: &Liability, today: Date) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let totals = &data.totals;
    let lead = lang.pick(
        "പൊതുമരാമത്ത് വകുപ്പിന്റെ ഒരു റോഡോ കെട്ടിടമോ പൂർത്തിയായാലും, നിശ്ചിത കാലത്തേക്ക് അതിലെ തകരാറുകൾ സ്വന്തം ചെലവിൽ പരിഹരിക്കാൻ കരാറുകാരന് ബാധ്യതയുണ്ട്. ആ കാലാവധിയിലുള്ള പ്രവൃത്തികൾ, വകുപ്പ് പ്രസിദ്ധീകരിച്ചതുപോലെ.",
        "When PWD finishes a road or a building, the contractor stays liable for a set period to repair defects at their own cost. These are the works inside that period, as PWD lists them.",
    );
    let with = |change: &dyn Fn(&mut LiabilityFilter)| {
        let mut next = filter.clone();
        next.page = 1;
        change(&mut next);
        href(lang, &next)
    };
    let path = href(Lang::Ml, filter);
    let pages = data.matching.div_ceil(LIABILITY_PAGE_SIZE).max(1);
    let chosen =
        data.rows.first().filter(|_| !filter.contractor.is_empty()).and_then(|row| row.contractor.as_deref()).map(contractor_display);

    let body = html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{p}/")) { (t.site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    span { (title(lang)) }
                }
                h1.d2 { (title(lang)) }
                p.lead { (lead) }
                form.tools method="get" action=(format!("{p}/liability")) {
                    div.filters {
                        div {
                            label for="district" { (lang.pick("പി.ഡബ്ല്യു.ഡി ഓഫീസ് ഉള്ള ജില്ല", "District of the PWD office")) }
                            select #district name="district" {
                                option value="" { (lang.pick("കേരളം മുഴുവൻ", "All of Kerala")) }
                                @for facet in data.districts.iter().filter(|f| !f.v.is_empty()) {
                                    option value=(facet.v) selected[facet.v == filter.district] { (district_label(lang, &facet.v)) " (" (facet.n) ")" }
                                }
                            }
                        }
                        div.go { button.btn type="submit" { (t.apply) } }
                    }
                }
            }
        }
        div.wrap {
            div.panels {
                div.panel.g {
                    h2 { (lang.pick("ഇപ്പോൾ ബാധ്യതാ കാലാവധിയിലുള്ളവ", "Works under liability now")) }
                    p.num { (totals.active) }
                    p.small {
                        @match lang {
                            Lang::Ml => { "പട്ടികയിൽ ആകെ " (totals.total) " പ്രവൃത്തികൾ" },
                            Lang::En => { (totals.total) " works on the list in all" },
                        }
                    }
                }
                div.panel {
                    h2 {
                        @match lang {
                            Lang::Ml => { (SOON_DAYS) " ദിവസത്തിനകം കാലാവധി തീരുന്നവ" },
                            Lang::En => { "Liability ends within " (SOON_DAYS) " days" },
                        }
                    }
                    p.num { (totals.ending_soon) }
                    p.small { (lang.pick("അതിനു ശേഷമുള്ള അറ്റകുറ്റപ്പണി പൊതുചെലവിലാകും.", "After that, repairs fall to the public purse.")) }
                }
                div.panel {
                    h2 { (lang.pick("കരാറുകാർ", "Contractors")) }
                    p.num { (totals.contractors) }
                    p.small { (lang.pick("പേരുകൾ വകുപ്പ് പ്രസിദ്ധീകരിച്ചതുപോലെ.", "Names as PWD publishes them.")) }
                }
            }

            div.note.sec {
                p {
                    b { (lang.pick("തകരാർ കണ്ടാൽ: ", "If you see a defect: ")) }
                    (lang.pick(
                        "പട്ടികയിലുള്ള ഒരു പ്രവൃത്തിയിൽ കാലാവധിക്കുള്ളിൽ തകരാർ വന്നാൽ, കരാറുകാരനെക്കൊണ്ട് അത് പരിഹരിപ്പിക്കാൻ വകുപ്പിന് കഴിയും. പരാതി പി.ഡബ്ല്യു.ഡിയെ അറിയിക്കുക; ബന്ധപ്പെട്ട ഓഫീസുകൾ വകുപ്പിന്റെ പട്ടികയിൽ ഉണ്ട്.",
                        "If a work on this list fails inside its period, PWD can make the contractor put it right. Report it to PWD; its own list names the office responsible for each work.",
                    ))
                    " "
                    a href=(dlp::SOURCE_URL) rel="noopener" { (lang.pick("വകുപ്പിന്റെ പട്ടിക", "PWD's list")) (icon(icons::ARROW_UP_RIGHT)) }
                }
                p.small {
                    (lang.pick(
                        "ജില്ല എന്നത് പ്രവൃത്തി കൈകാര്യം ചെയ്യുന്ന പി.ഡബ്ല്യു.ഡി ഓഫീസ് ഉള്ള ജില്ലയാണ്; ചില ഓഫീസുകൾ അയൽജില്ലകളിലെ പ്രവൃത്തികളും നോക്കുന്നു.",
                        "The district is that of the PWD office handling the work; some offices also look after works in neighbouring districts.",
                    ))
                }
            }

            div.result {
                div.chips {
                    a.chip.on[filter.wing.is_empty()] href=(with(&|next| next.wing.clear())) { (lang.pick("എല്ലാം", "All")) " " (totals.total) }
                    @for facet in &data.wings {
                        a.chip.on[facet.v == filter.wing] href=(with(&|next| next.wing = facet.v.clone())) { (wing_label(lang, &facet.v)) " " (facet.n) }
                    }
                    @if let Some(name) = &chosen {
                        a.chip.on href=(with(&|next| next.contractor.clear())) lang="en" { (name) (icon(icons::X)) }
                    }
                }
                span.small.muted { (lang.pick("കാലാവധി ആദ്യം തീരുന്നവ മുകളിൽ", "Soonest to end first")) }
            }

            @if data.rows.is_empty() {
                div.empty {
                    p { b { (lang.pick("ഈ തിരഞ്ഞെടുപ്പിൽ പ്രവൃത്തികളൊന്നുമില്ല.", "No works match this selection.")) } }
                    p.actions { a.btn.ghost href=(format!("{p}/liability")) { (lang.pick("എല്ലാം കാണുക", "Show all")) } }
                }
            } @else {
                ol.rows { @for row in &data.rows { (work_row(lang, row, today)) } }
                (pager(lang, filter.page, pages, &|n| with(&|next| next.page = n)))
            }

            @if filter.contractor.is_empty() && !data.contractors.is_empty() {
                section.sec {
                    div.sec-h {
                        div {
                            h2.d2 { (lang.pick("കൂടുതൽ പ്രവൃത്തികളുള്ള കരാറുകാർ", "Contractors with the most works")) }
                            p { (lang.pick(
                                "ഇപ്പോൾ ബാധ്യതാ കാലാവധിയിലുള്ള പ്രവൃത്തികളുടെ എണ്ണം. ഒരേ പേരിന്റെ വ്യത്യസ്ത അക്ഷരവിന്യാസങ്ങൾ ഞങ്ങൾ ഒന്നായി കണക്കാക്കുന്നു; ഒരേ പേരുള്ള രണ്ടു പേർ ഒന്നായി വന്നിരിക്കാം.",
                                "Counted by works now under liability. We treat different spellings of a name as one; two people with the same name may have been counted together.",
                            )) }
                        }
                        a.more href=(format!("{p}/contractors")) { (lang.pick("എല്ലാ കരാറുകാരും", "All contractors")) (icon(icons::ARROW_RIGHT)) }
                    }
                    div.tiles {
                        @for c in &data.contractors {
                            a.tile href=(format!("{p}/c/{}", c.contractor_key)) {
                                b lang="en" { (contractor_display(&c.contractor)) }
                                span.n { (c.n) small { (lang.pick("പ്രവൃത്തികൾ", "works")) } }
                            }
                        }
                    }
                }
            }

            section.sec #source {
                h2.h { (t.sources) }
                p.small.muted {
                    (lang.pick(
                        "ഉറവിടം: കേരള പൊതുമരാമത്ത് വകുപ്പിന്റെ ഡി.എൽ.പി വർക്ക് ലിസ്റ്റ്. ആഴ്ചയിലൊരിക്കൽ ഞങ്ങൾ അത് വായിക്കുന്നു. വകുപ്പിന്റെ താളിലുള്ള ഫോൺ നമ്പറുകൾ ഞങ്ങൾ സൂക്ഷിക്കുകയോ കാണിക്കുകയോ ചെയ്യുന്നില്ല.",
                        "Source: the Kerala Public Works Department's DLP work list, which we read once a week. We do not keep or show the phone numbers printed on PWD's pages.",
                    ))
                    " "
                    @match lang {
                        Lang::Ml => { "കരാർ തുക വകുപ്പിന്റെ താളിന്റെ കോഡിലുണ്ടെങ്കിലും താളിൽ കാണിക്കുന്നില്ല; ഇപ്പോൾ കാലാവധിയിലുള്ള " (totals.active) " പ്രവൃത്തികളിൽ " (totals.with_amount) " എണ്ണത്തിന് മാത്രമേ വകുപ്പ് അത് രേഖപ്പെടുത്തിയിട്ടുള്ളൂ." },
                        Lang::En => { "The agreed amount sits in the code of PWD's page without being displayed there, and PWD has filled it in for only " (totals.with_amount) " of the " (totals.active) " works now under liability." },
                    }
                }
                div.actions {
                    @if let Some(id) = totals.snapshot_id {
                        a.btn.ghost.sm href=(format!("/snapshot/{id}")) rel="nofollow" { (icon(icons::DOWNLOAD)) (lang.pick("ഞങ്ങൾ വായിച്ചതിന്റെ പകർപ്പ്", "Stored extract")) }
                    }
                    a.btn.ghost.sm href=(dlp::SOURCE_URL) rel="noopener" { (lang.pick("വകുപ്പിന്റെ പട്ടിക", "PWD's list")) (icon(icons::ARROW_UP_RIGHT)) }
                    a.btn.ghost.sm href="/api/v1/liability.csv" { (icon(icons::DATABASE)) "CSV" }
                }
            }
        }
    };

    layout(
        &Page {
            lang,
            title: title(lang),
            description: lead,
            path: &path,
            origin,
            nav: Nav::None,
            last_checked: totals.last_checked.as_deref(),
            head: html! {},
            image: None,
        },
        body,
    )
}

/// One work under liability. Also used on a contractor's page.
pub fn work_row(lang: Lang, row: &LiabilityRow, today: Date) -> Markup {
    let ends = row.ends_on.as_deref().and_then(Date::parse_iso);
    let starts = row.starts_on.as_deref().and_then(Date::parse_iso);
    let left = ends.map(|end| end.days_since(today));
    html! {
        li {
            div.row {
                span.ico { (icon(wing_icon(&row.wing))) }
                span.t lang="en" { (work_display(&row.name)) }
                span.m {
                    (wing_label(lang, &row.wing))
                    @if let Some(place) = row.subdivision.as_deref().or(row.division.as_deref()) { " · " (en(place)) }
                }
                span.amt {
                    @match (&row.contractor, &row.contractor_key) {
                        (Some(name), Some(key)) => {
                            a href=(format!("{}/c/{key}", lang.prefix())) lang="en" { b { (contractor_display(name)) } }
                        },
                        (Some(name), None) => b lang="en" { (contractor_display(name)) },
                        _ => span { (lang.pick("കരാറുകാരന്റെ പേര് പ്രസിദ്ധീകരിച്ചിട്ടില്ല", "Contractor not named")) },
                    }
                }
                span.m {
                    @match (starts, ends) {
                        (Some(starts), Some(ends)) => {
                            @match lang {
                                Lang::Ml => { "ബാധ്യത " (starts.to_dmy()) " മുതൽ " b { (ends.to_dmy()) } " വരെ" },
                                Lang::En => { "Liable from " (starts.to_dmy()) " until " b { (ends.to_dmy()) } },
                            }
                        },
                        (None, Some(ends)) => { (lang.pick("ബാധ്യത തീരുന്നത്", "Liable until")) " " b { (ends.to_dmy()) } },
                        _ => (lang.pick("തീയതികൾ പ്രസിദ്ധീകരിച്ചിട്ടില്ല", "Dates not published")),
                    }
                }
                @if let Some(amount) = row.agreed_amount {
                    span.m {
                        (lang.pick("കരാർ തുക", "Agreed amount")) " " b { (inr_short(amount, lang)) } " · " (inr(amount))
                    }
                }
                @if let Some(left) = left {
                    span.tags {
                        @if left < 0 {
                            span.badge { (lang.pick("കാലാവധി കഴിഞ്ഞു", "Period has ended")) }
                        } @else if left == 0 {
                            span.badge { (lang.pick("ഇന്ന് തീരും", "Ends today")) }
                        } @else if left <= SOON_DAYS {
                            span.badge {
                                @match lang { Lang::Ml => { (left) " ദിവസം ബാക്കി" }, Lang::En => { (left) (if left == 1 { " day left" } else { " days left" }) } }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn wing_icon(wing: &str) -> &'static str {
    match wing {
        "Roads" | "National Highways" | "KSTP" | "KRFB-PMU" | "RICK" => icons::ROUTE,
        "Buildings" => icons::BUILDING_2,
        _ => icons::LANDMARK,
    }
}

fn wing_label(lang: Lang, wing: &str) -> String {
    match wing {
        "Roads" => lang.pick("റോഡുകൾ", "Roads"),
        "Buildings" => lang.pick("കെട്ടിടങ്ങൾ", "Buildings"),
        "Bridges" => lang.pick("പാലങ്ങൾ", "Bridges"),
        "National Highways" => lang.pick("ദേശീയപാത", "National highways"),
        other => return other.to_string(),
    }
    .to_string()
}
