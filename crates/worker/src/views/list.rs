use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::inr_short;
use kanakku_core::i18n::{flag_label, Lang};
use kanakku_core::model::{headline, Headline};
use maud::{html, Markup};
use worker::url::form_urlencoded;

use super::{en, layout, Nav, Page};
use crate::db::{Facet, Filter, Listing, PAGE_SIZE};
use crate::http::encode_segment;

pub fn render(lang: Lang, filter: &Filter, listing: &Listing) -> String {
    let t = lang.t();
    let totals = &listing.totals;
    let pages = totals.total.div_ceil(PAGE_SIZE).max(1);
    let path = format!("/{}", query(filter, filter.page));
    let has_filters = !filter.department.is_empty() || !filter.constituency.is_empty() || !filter.status.is_empty() || !filter.flag.is_empty();

    let body = html! {
        h1 { (t.list_title) }
        p.intro { (t.list_intro) }

        p.figure {
            b { (totals.total) }
            span {
                (t.projects_found)
                @if totals.flagged > 0 {
                    " · "
                    a.flagged href=(format!("{}/{}", lang.prefix(), query(&Filter { flag: "any".into(), page: 1, ..filter.clone() }, 1))) {
                        @match lang {
                            Lang::Ml => { (totals.flagged) " എണ്ണത്തിന് സൂചനയുണ്ട്" },
                            Lang::En => { (totals.flagged) " with a flag" },
                        }
                    }
                }
            }
        }

        form.find method="get" action=(format!("{}/", lang.prefix())) role="search" {
            label for="q" { (t.search_label) }
            div.search {
                input #q type="search" name="q" value=(filter.q) placeholder=(t.search_placeholder) autocomplete="off" enterkeyhint="search";
                button type="submit" { (t.search_label) }
            }
            details open[has_filters] {
                summary { (t.filters) }
                div.filters {
                    (select(lang, "dept", t.filter_department, &listing.departments, &filter.department))
                    (select(lang, "lac", t.filter_constituency, &listing.constituencies, &filter.constituency))
                    (select(lang, "status", t.filter_status, &listing.statuses, &filter.status))
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
                    div.go {
                        button type="submit" { (t.apply) }
                        @if !filter.is_empty() {
                            a href=(format!("{}/", lang.prefix())) { (t.clear) }
                        }
                    }
                }
            }
        }

        @if listing.rows.is_empty() {
            p.empty { (t.no_results) }
        } @else {
            ol.ledger {
                @for row in &listing.rows {
                    li {
                        a href=(format!("{}/p/{}", lang.prefix(), encode_segment(&row.code))) {
                            span.t lang="en" { (row.title_en) }
                            span.amt {
                                @match headline(row.estimated_amount, row.estimate_shared_by, row.expenditure, row.works_amount) {
                                    Some((amount, kind)) => { (inr_short(amount, lang)) small { (headline_label(lang, kind)) } },
                                    None => span.none { "—" },
                                }
                            }
                            span.meta {
                                span.code { (row.code) }
                                @if let Some(agency) = &row.executing_agency { " · " (en(agency)) }
                                @if let Some(names) = &row.constituencies { " · " (en(names)) }
                                @if let Some(status) = &row.official_status { " · " (en(status)) }
                            }
                            @if let Some(types) = &row.flag_types {
                                span.tags {
                                    @for kind in types.split(',').filter_map(FlagKind::parse) {
                                        span.tag { (flag_label(lang, kind)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            @if pages > 1 {
                nav.pager aria-label=(t.page) {
                    @if filter.page > 1 {
                        a rel="prev" href=(format!("{}/{}", lang.prefix(), query(filter, filter.page - 1))) { "← " (t.previous) }
                    } @else { span {} }
                    span.small { (t.page) " " (filter.page) " / " (pages) }
                    @if filter.page < pages {
                        a rel="next" href=(format!("{}/{}", lang.prefix(), query(filter, filter.page + 1))) { (t.next) " →" }
                    } @else { span {} }
                }
            }
        }
    };

    layout(
        &Page {
            lang,
            title: t.list_title,
            description: t.list_intro,
            path: &path,
            nav: Nav::Projects,
            last_checked: totals.last_checked.as_deref(),
            head: html! {},
        },
        body,
    )
}

/// Says what a row's figure is, because it differs between rows.
pub fn headline_label(lang: Lang, kind: Headline) -> &'static str {
    match kind {
        Headline::Estimate => lang.pick("കണക്കാക്കിയ തുക", "estimate"),
        Headline::Spent => lang.pick("ഇതുവരെ ചെലവ്", "spent so far"),
        Headline::WorksTotal => lang.pick("പ്രവൃത്തികളുടെ ആകെ", "total of works"),
    }
}

fn select(lang: Lang, name: &str, label: &str, facets: &[Facet], current: &str) -> Markup {
    html! {
        div {
            label for=(name) { (label) }
            select id=(name) name=(name) {
                option value="" { (lang.t().filter_all) }
                @for f in facets {
                    option value=(f.v) selected[f.v == current] lang="en" { (f.v) " (" (f.n) ")" }
                }
            }
        }
    }
}

/// The query string for a filter and page, without empty parameters. Empty when nothing is set.
fn query(filter: &Filter, page: u32) -> String {
    let mut s = form_urlencoded::Serializer::new(String::new());
    for (key, value) in [
        ("q", &filter.q),
        ("dept", &filter.department),
        ("lac", &filter.constituency),
        ("status", &filter.status),
        ("flag", &filter.flag),
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
