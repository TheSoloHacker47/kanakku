//! The project list: search, filters, sorting, and a summary of whatever is selected.

use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::inr_short;
use kanakku_core::i18n::{flag_label, Lang};
use kanakku_core::names::{constituency_label, department_label};
use kanakku_core::stage::Stage;
use maud::{html, Markup};
use worker::url::form_urlencoded;

use super::{icon, layout, project_row, Nav, Page};
use crate::db::{Filter, Listing, Sort, PAGE_SIZE};
use crate::icons;

pub fn render(lang: Lang, origin: &str, filter: &Filter, listing: &Listing) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let totals = &listing.totals;
    let pages = totals.total.div_ceil(PAGE_SIZE).max(1);
    let path = format!("/projects{}", query(filter, filter.page));
    let link = |f: &Filter, page: u32| format!("{p}/projects{}", query(f, page));

    // The heading names what is selected, when one thing is.
    let constituency = listing.constituencies.iter().find(|c| c.v == filter.constituency);
    let heading = if !filter.constituency.is_empty() {
        constituency_label(lang, &filter.constituency).to_string()
    } else if !filter.department.is_empty() {
        department_label(lang, &filter.department)
    } else if filter.flag == "any" {
        lang.pick("സൂചനയുള്ള പദ്ധതികൾ", "Flagged projects").to_string()
    } else {
        t.nav_projects.to_string()
    };
    let mla = constituency.and_then(|c| match lang {
        Lang::Ml => c.mla_ml.as_deref().or(c.mla.as_deref()),
        Lang::En => c.mla.as_deref(),
    });

    let body = html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{p}/")) { (t.site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    @if heading == t.nav_projects { span { (t.nav_projects) } } @else { a href=(format!("{p}/projects")) { (t.nav_projects) } }
                }
                h1.d2 { (heading) }
                p.lead {
                    @match lang {
                        Lang::Ml => { b { (totals.total) } " പദ്ധതികൾ" },
                        Lang::En => { b { (totals.total) } " projects" },
                    }
                    @if let Some(spent) = totals.spent.filter(|s| *s > 0) {
                        " · " b { (inr_short(spent, lang)) } " " (lang.pick("ഇതുവരെ ചെലവ്", "spent so far"))
                    }
                    @if totals.flagged > 0 && filter.flag.is_empty() {
                        " · "
                        a href=(link(&Filter { flag: "any".into(), ..filter.clone() }, 1)) {
                            @match lang {
                                Lang::Ml => { (totals.flagged) " എണ്ണത്തിന് സൂചന" },
                                Lang::En => { (totals.flagged) " flagged" },
                            }
                        }
                    }
                }
                @if let Some(mla) = mla {
                    p.small { (t.mla) " (" (lang.pick("കിഫ്ബി പട്ടികപ്രകാരം", "as KIIFB lists")) "): " b { (mla) } }
                }

                form.tools method="get" action=(format!("{p}/projects")) role="search" {
                    div.find {
                        div.field {
                            (icon(icons::SEARCH))
                            label.vh for="q" { (t.search_label) }
                            input #q type="search" name="q" value=(filter.q) placeholder=(t.search_placeholder) autocomplete="off" enterkeyhint="search";
                        }
                        button.btn type="submit" { (t.search_label) }
                    }
                    details {
                        summary.btn.ghost.sm { (icon(icons::SLIDERS_HORIZONTAL)) (lang.pick("അരിപ്പകളും ക്രമവും", "Filter and sort")) }
                        div.filters {
                            div {
                                label for="dept" { (t.filter_department) }
                                select #dept name="dept" {
                                    option value="" { (t.filter_all) }
                                    @for f in &listing.departments {
                                        option value=(f.v) selected[f.v == filter.department] { (department_label(lang, &f.v)) " (" (f.n) ")" }
                                    }
                                }
                            }
                            div {
                                label for="lac" { (t.filter_constituency) }
                                select #lac name="lac" {
                                    option value="" { (t.filter_all) }
                                    @for f in &listing.constituencies {
                                        option value=(f.v) selected[f.v == filter.constituency] { (constituency_label(lang, &f.v)) " (" (f.n) ")" }
                                    }
                                }
                            }
                            div {
                                label for="stage" { (lang.pick("ഘട്ടം", "Stage")) }
                                select #stage name="stage" {
                                    option value="" { (t.filter_all) }
                                    @for stage in Stage::ALL {
                                        @if let Some(f) = listing.stages.iter().find(|f| f.v == stage.as_str()) {
                                            option value=(stage.as_str()) selected[filter.stage == stage.as_str()] { (stage.label(lang)) " (" (f.n) ")" }
                                        }
                                    }
                                }
                            }
                            div {
                                label for="flag" { (t.filter_flag) }
                                select #flag name="flag" {
                                    option value="" { (t.filter_all) }
                                    option value="any" selected[filter.flag == "any"] { (t.filter_any_flag) }
                                    @for kind in FlagKind::ALL {
                                        option value=(kind.as_str()) selected[filter.flag == kind.as_str()] { (flag_label(lang, kind)) }
                                    }
                                }
                            }
                            div {
                                label for="sort" { (lang.pick("ക്രമം", "Sort by")) }
                                select #sort name="sort" {
                                    @for sort in Sort::ALL {
                                        option value=(sort.as_str()) selected[filter.sort == sort] { (sort_label(lang, sort)) }
                                    }
                                }
                            }
                            div.go { button.btn type="submit" { (t.apply) } }
                        }
                    }
                }
            }
        }

        div.wrap {
            div.result {
                div.chips {
                    @if !filter.q.is_empty() {
                        (chip(&format!("“{}”", filter.q), &link(&Filter { q: String::new(), ..filter.clone() }, 1)))
                    }
                    @if !filter.department.is_empty() {
                        (chip(&department_label(lang, &filter.department), &link(&Filter { department: String::new(), ..filter.clone() }, 1)))
                    }
                    @if !filter.constituency.is_empty() {
                        (chip(constituency_label(lang, &filter.constituency), &link(&Filter { constituency: String::new(), ..filter.clone() }, 1)))
                    }
                    @if let Some(stage) = Stage::parse(&filter.stage) {
                        (chip(stage.label(lang), &link(&Filter { stage: String::new(), ..filter.clone() }, 1)))
                    }
                    @if filter.flag == "any" {
                        (chip(t.filter_any_flag, &link(&Filter { flag: String::new(), ..filter.clone() }, 1)))
                    } @else if let Some(kind) = FlagKind::parse(&filter.flag) {
                        (chip(flag_label(lang, kind), &link(&Filter { flag: String::new(), ..filter.clone() }, 1)))
                    }
                    @if !filter.is_empty() {
                        a.small href=(format!("{p}/projects")) { (lang.pick("എല്ലാം മായ്ക്കുക", "Clear all")) }
                    }
                }
                span.small.muted { (lang.pick("ക്രമം", "Sorted by")) ": " (sort_label(lang, filter.sort)) }
            }

            @if listing.rows.is_empty() {
                div.empty {
                    p { b { (t.no_results) } }
                    p.muted { (lang.pick("അക്ഷരത്തെറ്റ് പരിശോധിക്കുക, അല്ലെങ്കിൽ അരിപ്പകൾ മാറ്റിനോക്കുക.", "Check the spelling, or remove a filter.")) }
                    p.actions { a.btn.ghost href=(format!("{p}/projects")) { (lang.pick("എല്ലാ പദ്ധതികളും കാണുക", "Show all projects")) } }
                }
            } @else {
                ol.rows { @for row in &listing.rows { (project_row(lang, row)) } }
                @if pages > 1 {
                    nav.pager aria-label=(t.page) {
                        @if filter.page > 1 {
                            a.btn.ghost.sm rel="prev" href=(link(filter, filter.page - 1)) { (icon(icons::ARROW_LEFT)) (t.previous) }
                        } @else { span {} }
                        span.small.muted { (t.page) " " (filter.page) " / " (pages) }
                        @if filter.page < pages {
                            a.btn.ghost.sm rel="next" href=(link(filter, filter.page + 1)) { (t.next) (icon(icons::ARROW_RIGHT)) }
                        } @else { span {} }
                    }
                }
            }
        }
    };

    let description = match lang {
        Lang::Ml => format!("{heading}: {} പദ്ധതികൾ. {}", totals.total, t.list_intro),
        Lang::En => format!("{heading}: {} projects. {}", totals.total, t.list_intro),
    };
    layout(
        &Page {
            lang,
            title: &heading,
            description: &description,
            path: &path,
            origin,
            nav: Nav::Projects,
            last_checked: totals.last_checked.as_deref(),
            head: html! {},
        },
        body,
    )
}

/// An active filter, removable with one tap.
fn chip(label: &str, remove: &str) -> Markup {
    html! { a.chip href=(remove) { (label) (icon(icons::X)) } }
}

fn sort_label(lang: Lang, sort: Sort) -> &'static str {
    match sort {
        Sort::Flags => lang.pick("സൂചനയുള്ളവ ആദ്യം", "Flagged first"),
        Sort::Amount => lang.pick("വലിയ തുക ആദ്യം", "Largest amount"),
        Sort::Spent => lang.pick("കൂടുതൽ ചെലവായവ ആദ്യം", "Most spent"),
        Sort::Name => lang.pick("പേര് (A–Z)", "Name (A–Z)"),
        Sort::Changed => lang.pick("ഈയിടെ മാറിയവ ആദ്യം", "Recently changed"),
    }
}

/// The query string for a filter and page, without empty parameters. Empty when nothing is set.
fn query(filter: &Filter, page: u32) -> String {
    let mut s = form_urlencoded::Serializer::new(String::new());
    for (key, value) in [
        ("q", filter.q.as_str()),
        ("dept", filter.department.as_str()),
        ("lac", filter.constituency.as_str()),
        ("stage", filter.stage.as_str()),
        ("flag", filter.flag.as_str()),
        ("sort", if filter.sort == Sort::Flags { "" } else { filter.sort.as_str() }),
    ] {
        if !value.is_empty() {
            s.append_pair(key, value);
        }
    }
    if page > 1 {
        s.append_pair("page", &page.to_string());
    }
    let out = s.finish();
    if out.is_empty() {
        out
    } else {
        format!("?{out}")
    }
}
