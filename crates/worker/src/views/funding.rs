//! What KIIFB approved and what it has released: the list, one project, and the section shown on a map package's page.

use kanakku_core::entity::{agency_display, agency_key};
use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::{inr, inr_short, pct};
use kanakku_core::i18n::{flag_explain, flag_label, flag_rule, Lang};
use kanakku_core::kiifb_status::{self as status, Basis, FundedProject, FundedWork};
use kanakku_core::names::{department_label, district_label, districts};
use kanakku_core::title::display_title;
use kanakku_core::Date;
use maud::{html, Markup};

use super::{big_amount, dash, department_icon, en, icon, layout, meter, pager, pair, Nav, Page};
use crate::db::{FundingLink, FundingListing, FundingPage, FundingRow, FundingSort, FundingTotals, FUNDING_PAGE_SIZE};
use crate::http::encode_segment;
use crate::icons;

pub fn title(lang: Lang) -> &'static str {
    lang.pick("അനുവദിച്ചതും നൽകിയതും", "Approved and paid")
}

/// What narrows the list, as it appears in the address.
#[derive(Clone, Debug, Default)]
pub struct Query {
    pub sort: FundingSort,
    pub district: String,
    pub q: String,
    pub flagged: bool,
    pub page: u32,
}

impl Query {
    fn href(&self, lang: Lang) -> String {
        let mut parts = Vec::new();
        if !self.q.is_empty() {
            parts.push(pair("q", &self.q));
        }
        if !self.district.is_empty() {
            parts.push(pair("district", &self.district));
        }
        if self.flagged {
            parts.push("flag=1".to_string());
        }
        if self.sort != FundingSort::default() {
            parts.push(format!("sort={}", self.sort.as_str()));
        }
        if self.page > 1 {
            parts.push(format!("page={}", self.page));
        }
        let query = if parts.is_empty() { String::new() } else { format!("?{}", parts.join("&")) };
        format!("{}/funding{query}", lang.prefix())
    }
}

pub fn list(lang: Lang, origin: &str, query: &Query, listing: &FundingListing) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let totals = &listing.totals;
    let place = if query.district.is_empty() { lang.pick("കേരളം", "Kerala") } else { district_label(lang, &query.district) };
    let lead = match lang {
        Lang::Ml => format!(
            "കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളിൽ {place} എന്നതിന് കീഴിൽ {} പദ്ധതികളുണ്ട്. ഓരോന്നിനും കിഫ്ബി അനുവദിച്ച തുകയും ഇതുവരെ നൽകിയ തുകയും ഇവിടെ കാണാം.",
            totals.total
        ),
        Lang::En => format!(
            "KIIFB's project status page lists {} projects under {place}. For each one, this is what KIIFB approved and what has been paid so far.",
            totals.total
        ),
    };
    let pages = listing.matching.div_ceil(FUNDING_PAGE_SIZE).max(1);
    let path = query.href(Lang::Ml);
    let with = |change: &dyn Fn(&mut Query)| {
        let mut next = query.clone();
        next.page = 1;
        change(&mut next);
        next.href(lang)
    };

    let body = html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{p}/")) { (t.site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    span { (title(lang)) }
                }
                h1.d2 { (title(lang)) @if !query.district.is_empty() { " · " (place) } }
                p.lead { (lead) }
                form.tools method="get" action=(format!("{p}/funding")) role="search" {
                    div.find {
                        div.field {
                            (icon(icons::SEARCH))
                            label.vh for="q" { (t.search_label) }
                            input #q type="search" name="q" value=(query.q) placeholder=(lang.pick("പദ്ധതിയുടെയോ സ്ഥാപനത്തിന്റെയോ പേര്", "Project or agency name")) autocomplete="off" enterkeyhint="search";
                        }
                        button.btn type="submit" { (t.search_label) }
                    }
                    div.filters {
                        div {
                            label for="district" { (lang.pick("ജില്ല", "District")) }
                            select #district name="district" {
                                option value="" { (lang.pick("കേരളം മുഴുവൻ", "All of Kerala")) }
                                @for name in districts() {
                                    option value=(name) selected[name == query.district] { (district_label(lang, name)) }
                                }
                            }
                        }
                        div {
                            label for="sort" { (lang.pick("ക്രമം", "Sort by")) }
                            select #sort name="sort" {
                                @for option in FundingSort::ALL {
                                    option value=(option.as_str()) selected[option == query.sort] { (sort_label(lang, option)) }
                                }
                            }
                        }
                        div.go { button.btn type="submit" { (t.apply) } }
                    }
                }
            }
        }
        div.wrap {
            (totals_panels(lang, totals, None))
            @if query.district.is_empty() {
                @if totals.unread > 0 { (unread_note(lang, totals.unread)) }
            } @else {
                p.note.sec {
                    (lang.pick(
                        "ചില പദ്ധതികൾ പല ജില്ലകളിലായുള്ളവയാണ് (ഉദാ: സ്കൂൾ ക്ലസ്റ്ററുകൾ). അതിനാൽ ഈ ആകെത്തുക ഈ ജില്ലയ്ക്ക് മാത്രമുള്ളതല്ല.",
                        "Some projects span several districts, school clusters for example. So these totals are not this district's alone.",
                    ))
                }
                @if totals.unread > 0 { (unread_note(lang, totals.unread)) }
            }

            div.result {
                div.chips {
                    @if !query.q.is_empty() {
                        a.chip href=(with(&|next| next.q.clear())) { "“" (query.q) "”" (icon(icons::X)) }
                    }
                    @if !query.district.is_empty() {
                        a.chip href=(with(&|next| next.district.clear())) { (place) (icon(icons::X)) }
                    }
                    @if totals.flagged > 0 || query.flagged {
                        a.chip.on[query.flagged] href=(with(&|next| next.flagged = !query.flagged)) {
                            (icon(icons::FLAG))
                            @match lang { Lang::Ml => { (totals.flagged) " എണ്ണത്തിന് സൂചന" }, Lang::En => { (totals.flagged) " flagged" } }
                        }
                    }
                }
                span.small.muted {
                    @match lang {
                        Lang::Ml => { (listing.matching) " പദ്ധതികൾ · " (sort_label(lang, query.sort)) },
                        Lang::En => { (listing.matching) " projects · " (sort_label(lang, query.sort)) },
                    }
                }
            }
            @if listing.rows.is_empty() {
                div.empty {
                    p { b { (t.no_results) } }
                    p.actions { a.btn.ghost href=(format!("{p}/funding")) { (lang.pick("എല്ലാം കാണുക", "Show all")) } }
                }
            } @else {
                ol.rows { @for row in &listing.rows { (list_row(lang, row)) } }
                (pager(lang, query.page, pages, &|n| with(&|next| next.page = n)))
            }
        }
    };
    layout(
        &Page {
            lang,
            title: title(lang),
            description: &lead,
            path: &path,
            origin,
            nav: Nav::Funding,
            last_checked: totals.last_checked.as_deref(),
            head: html! {},
            image: None,
        },
        body,
    )
}

fn unread_note(lang: Lang, unread: u32) -> Markup {
    html! {
        p.note.sec {
            @match lang {
                Lang::Ml => { (unread) " പദ്ധതികളുടെ പ്രവൃത്തിപ്പട്ടിക ഇതുവരെ വായിച്ചിട്ടില്ല; ഓരോ രാത്രിയും കുറച്ചെണ്ണം വീതം വായിക്കുന്നു. അവയ്ക്ക് കാണിക്കുന്ന “നൽകിയ തുക” കിഫ്ബിയുടെ പട്ടികയിലേതാണ്; പല ജില്ലകളിലായുള്ള പദ്ധതികൾക്ക് അത് യഥാർത്ഥത്തേക്കാൾ കൂടുതലായിരിക്കാം." },
                Lang::En => { "The work tables of " (unread) " projects have not been read yet; a batch is read each night. Until then their “paid” figure is the one on KIIFB's list, which overstates it for projects that span districts." },
            }
        }
    }
}

/// The three headline blocks: approved, paid, and how many projects. `more` links to the full list.
pub fn totals_panels(lang: Lang, totals: &FundingTotals, more: Option<&str>) -> Markup {
    let approved = totals.approved.unwrap_or(0);
    let released = totals.released.unwrap_or(0);
    html! {
        div.panels.flat[more.is_some()] {
            div.panel.g {
                h2 { (lang.pick("കിഫ്ബി അനുവദിച്ചത്", "Approved by KIIFB")) }
                p.num { (big_amount(approved, lang)) }
                p.small { (inr(approved)) }
            }
            div.panel {
                h2 { (lang.pick("ഇതുവരെ നൽകിയത്", "Paid so far")) }
                p.num { (big_amount(released, lang)) }
                @if approved > 0 && released <= approved {
                    p.small { (share_of_approved(lang, released, approved)) }
                    (meter(released as f64 / approved as f64, ""))
                } @else {
                    p.small { (inr(released)) }
                }
                @if totals.unread > 0 {
                    p.small.muted {
                        @match lang {
                            Lang::Ml => { "താൽക്കാലികം: " (totals.unread) " പദ്ധതികളുടെ പ്രവൃത്തിപ്പട്ടിക ഇനിയും വായിക്കാനുണ്ട്." },
                            Lang::En => { "Provisional: the work tables of " (totals.unread) " projects are still to be read." },
                        }
                    }
                }
            }
            div.panel {
                h2 { (lang.pick("പദ്ധതികളും പ്രവൃത്തികളും", "Projects and works")) }
                p.num { (totals.total) }
                p.small {
                    @match lang {
                        Lang::Ml => { (totals.works) " പ്രവൃത്തികൾ · " (totals.evaluating) " പദ്ധതികൾ പരിശോധനയിൽ (അനുമതിയായിട്ടില്ല)" },
                        Lang::En => { (totals.works) " works · " (totals.evaluating) " projects still under evaluation" },
                    }
                }
                @if let Some(href) = more {
                    p { a.more href=(href) { (lang.pick("എല്ലാം കാണുക", "See them all")) (icon(icons::ARROW_RIGHT)) } }
                }
            }
        }
    }
}

pub fn list_row(lang: Lang, row: &FundingRow) -> Markup {
    html! {
        li {
            a.row href=(format!("{}/f/{}", lang.prefix(), encode_segment(&row.reference))) {
                span.ico { (icon(department_icon(row.department.as_deref()))) }
                span.t lang="en" { (display_title(&row.name)) }
                span.m {
                    @if let Some(department) = &row.department { (department_label(lang, department)) }
                    @if let Some(spv) = &row.spv { " · " (en(&agency_display(spv))) }
                }
                span.amt {
                    @match (row.approved_amount, row.released_amount) {
                        (Some(approved), released) => {
                            b { (inr_short(released.unwrap_or(0), lang)) }
                            span {
                                @match lang {
                                    Lang::Ml => { "നൽകി · അനുവദിച്ചത് " (inr_short(approved, lang)) },
                                    Lang::En => { "paid of " (inr_short(approved, lang)) " approved" },
                                }
                            }
                        },
                        (None, _) => span { (lang.pick("അനുമതിയായിട്ടില്ല; പരിശോധനയിൽ", "Not approved yet; under evaluation")) },
                    }
                }
                @if let (Some(approved), Some(released)) = (row.approved_amount.filter(|a| *a > 0), row.released_amount) {
                    span.bar { (meter(released as f64 / approved as f64, "")) }
                }
                span.tags {
                    @if row.flag_count > 0 {
                        span.badge.flag { (icon(icons::FLAG)) (flag_label(lang, FlagKind::PaidAboveApproval)) }
                    }
                    @if row.work_count > 0 {
                        span.badge {
                            @match lang {
                                Lang::Ml => { (row.work_count) (if row.work_count == 1 { " പ്രവൃത്തി" } else { " പ്രവൃത്തികൾ" }) },
                                Lang::En => { (row.work_count) (if row.work_count == 1 { " work" } else { " works" }) },
                            }
                        }
                    }
                    @if row.read == 0 { span.badge { (lang.pick("പ്രവൃത്തികൾ വായിച്ചിട്ടില്ല", "Works not read yet")) } }
                    @if row.packages > 0 { span.badge { (icon(icons::MAP_PIN)) (lang.pick("ഭൂപടത്തിലുണ്ട്", "On the map")) } }
                }
            }
        }
    }
}

pub fn detail(lang: Lang, origin: &str, project: &FundedProject, data: &FundingPage) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let row = &data.row;
    let name = display_title(&project.name);
    let path = format!("/f/{}", encode_segment(&project.reference));
    let listed = Date::from_utc_timestamp_ist(&row.fetched_at).map(Date::to_dmy);
    let works_checked = row.detail_checked_at.as_deref().and_then(Date::from_utc_timestamp_ist).map(Date::to_dmy);
    let first_seen = Date::parse_iso(&row.first_seen_on).map(Date::to_dmy);
    let works_approved: i64 = project.works.iter().filter_map(|w| w.approved).sum();
    let works_paid = project.works_paid();

    let body = html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{p}/")) { (t.site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    a href=(format!("{p}/funding")) { (title(lang)) }
                }
                div.meta {
                    span.ico { (icon(department_icon(project.department.as_deref()))) }
                    @if let Some(department) = &project.department { span { (department_label(lang, department)) } }
                    @if let Some(status) = &project.status { span.badge { (status_label(lang, status)) } }
                }
                h1 lang="en" { (name) }
            }
        }

        div.wrap {
            div.panels {
                div.panel.g {
                    h2 { (lang.pick("കിഫ്ബി അനുവദിച്ചത്", "Approved by KIIFB")) }
                    @match project.approved {
                        Some(approved) => { p.num { (big_amount(approved, lang)) } p.small { (inr(approved)) } },
                        None => {
                            p.num { (dash(lang)) }
                            p.small { (lang.pick("ഈ പദ്ധതിക്ക് ഇതുവരെ അനുമതിയായിട്ടില്ല.", "This project has not been approved yet.")) }
                        },
                    }
                }
                div.panel {
                    h2 { (lang.pick("ഇതുവരെ നൽകിയത്", "Paid so far")) }
                    @match project.paid() {
                        Some(paid) => {
                            p.num { (big_amount(paid, lang)) }
                            p.small {
                                (inr(paid))
                                @if let Some(approved) = project.approved.filter(|a| *a > 0 && paid <= *a) {
                                    " · " (share_of_approved(lang, paid, approved))
                                }
                            }
                            @if let Some(approved) = project.approved.filter(|a| *a > 0 && paid <= *a) { (meter(paid as f64 / approved as f64, "")) }
                        },
                        None => { p.num aria-hidden="true" { "—" } p.small { (t.not_reported) } },
                    }
                }
                div.panel {
                    h2 { (lang.pick("പ്രവൃത്തികൾ", "Works")) }
                    p.num { (project.works.len()) }
                    p.small {
                        @if project.works.is_empty() {
                            (lang.pick("കിഫ്ബി ഈ പദ്ധതിക്ക് കീഴിൽ പ്രവൃത്തികളൊന്നും പട്ടികപ്പെടുത്തിയിട്ടില്ല.", "KIIFB lists no works under this project."))
                        } @else {
                            @match lang {
                                Lang::Ml => { "പ്രവൃത്തികൾക്ക് ആകെ നൽകിയത് " (inr_short(works_paid, lang)) },
                                Lang::En => { (inr_short(works_paid, lang)) " paid across the works" },
                            }
                        }
                    }
                }
            }

            @if let Some(since) = row.missing_since.as_deref().and_then(Date::parse_iso) {
                p.note.warn.sec {
                    @match lang {
                        Lang::Ml => { (since.to_dmy()) " മുതൽ ഈ പദ്ധതി കിഫ്ബിയുടെ സ്റ്റാറ്റസ് താളിൽ കാണുന്നില്ല. താഴെയുള്ളത് അവസാനം രേഖപ്പെടുത്തിയ വിവരങ്ങളാണ്." },
                        Lang::En => { "This project has not appeared on KIIFB's status page since " (since.to_dmy()) ". The figures below are the last ones recorded." },
                    }
                }
            }

            (source_notes(lang, project))

            @if !data.flags.is_empty() {
                section.sec #flags {
                    h2.h { (t.flags) }
                    @for flag in &data.flags {
                        @if let Some(kind) = FlagKind::parse(&flag.kind) {
                            @let value: serde_json::Value = serde_json::from_str(&flag.value_json).unwrap_or_default();
                            div.flagbox.off[flag.status != "open"] {
                                (icon(icons::FLAG))
                                h3 {
                                    (flag_label(lang, kind))
                                    @if let Some(work) = &flag.work_ref { " · " (en(&display_title(work))) }
                                }
                                p { (flag_explain(lang, kind, &value)) }
                                p.small {
                                    (lang.pick("നിയമം", "Rule")) ": " (flag_rule(lang, kind)) " "
                                    a href=(format!("{p}/methodology#rules")) { (t.rule_version) " " (flag.rule_version) }
                                    @if let Some(cleared) = flag.cleared_on.as_deref().and_then(Date::parse_iso) {
                                        " · "
                                        @match lang {
                                            Lang::Ml => { (cleared.to_dmy()) "-ന് ഈ സൂചന നീങ്ങി" },
                                            Lang::En => { "Cleared on " (cleared.to_dmy()) },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            @if !project.works.is_empty() {
                section.sec {
                    h2.h { (lang.pick("പ്രവൃത്തികൾ", "Works")) " (" (project.works.len()) ")" }
                    @for (i, w) in project.works.iter().enumerate() {
                        @let package = data.links.iter().find(|l| l.seq as usize == i + 1).map(|l| l.project_code.as_str());
                        (work(lang, w, package))
                    }
                    @if project.works.len() > 1 {
                        p.small.muted {
                            @match lang {
                                Lang::Ml => { "പ്രവൃത്തികളുടെ ആകെ: അനുവദിച്ചത് " (inr(works_approved)) ", നൽകിയത് " (inr(works_paid)) "." },
                                Lang::En => { "Total of the works: " (inr(works_approved)) " approved, " (inr(works_paid)) " paid." },
                            }
                        }
                    }
                }
            }

            div.cols.sec {
                section {
                    h2.h { (lang.pick("വിശദാംശങ്ങൾ", "Details")) }
                    dl.kv {
                        @if let Some(department) = &project.department {
                            div { dt { (t.department) } dd { (department_label(lang, department)) } }
                        }
                        @if let Some(spv) = &project.spv {
                            div {
                                dt { (lang.pick("നിർവഹണ സ്ഥാപനം (SPV)", "Implementing agency (SPV)")) }
                                dd lang="en" { a href=(format!("{p}/a/{}", agency_key(spv))) { (agency_display(spv)) } }
                            }
                        }
                        @if let Some(main) = &project.main_project {
                            div { dt { (lang.pick("ഏത് പ്രഖ്യാപനത്തിന് കീഴിൽ", "Announced under")) } dd lang="en" { (main) } }
                        }
                        @if let Some(status) = &project.status {
                            div { dt { (lang.pick("കിഫ്ബി നില", "KIIFB status")) } dd { (status_label(lang, status)) } }
                        }
                        @if name != project.name {
                            div.long { dt { (lang.pick("കിഫ്ബി പ്രസിദ്ധീകരിച്ച പേര്", "Name as KIIFB publishes it")) } dd lang="en" { (project.name) } }
                        }
                    }
                }
                section #source {
                    h2.h { (t.sources) }
                    dl.kv {
                        div { dt { (t.sources) } dd { (lang.pick("കിഫ്ബി പ്രോജക്ട് സ്റ്റാറ്റസ്", "KIIFB project status")) small { "kiifb.org/prjStatus.jsp" } } }
                        @if let Some(date) = &listed { div { dt { (lang.pick("തുകകൾ ശേഖരിച്ചത്", "Amounts retrieved")) } dd { (date) } } }
                        @if let Some(date) = &works_checked { div { dt { (lang.pick("പ്രവൃത്തികൾ പരിശോധിച്ചത്", "Works last checked")) } dd { (date) } } }
                        @if let Some(date) = &first_seen { div { dt { (t.first_seen) } dd { (date) } } }
                    }
                    div.actions {
                        a.btn.ghost.sm href=(format!("/snapshot/{}", row.snapshot_id)) rel="nofollow" { (icon(icons::DOWNLOAD)) (lang.pick("പട്ടികയുടെ പകർപ്പ്", "Stored copy of the list")) }
                        @if let Some(id) = row.detail_snapshot_id {
                            a.btn.ghost.sm href=(format!("/snapshot/{id}")) rel="nofollow" { (icon(icons::DOWNLOAD)) (lang.pick("പ്രവൃത്തി പട്ടികയുടെ പകർപ്പ്", "Stored copy of the works")) }
                        }
                        a.btn.ghost.sm href=(status::SOURCE_URL) rel="noopener" { (lang.pick("കിഫ്ബി താൾ", "KIIFB page")) (icon(icons::ARROW_UP_RIGHT)) }
                    }
                }
            }

            @if !data.packages.is_empty() {
                section.sec {
                    h2.h { (lang.pick("ഭൂപട ഡാഷ്ബോർഡിലെ ഇതേ പദ്ധതി", "The same project on the map dashboard")) }
                    p.small.muted { (basis_note(lang, row.match_basis.as_deref().and_then(Basis::parse))) }
                    ul.list {
                        @for s in &data.packages {
                            li {
                                a href=(format!("{p}/p/{}", encode_segment(&s.code))) lang="en" { (display_title(&s.title_en)) }
                                " " span.small.muted {
                                    span.code { (s.code) }
                                    @if let Some(spent) = s.expenditure { " · " (inr_short(spent, lang)) " " (lang.pick("ചെലവ്", "spent")) }
                                }
                                @if s.flag_count > 0 { " " span.badge.flag { (icon(icons::FLAG)) (s.flag_count) span.vh { " " (lang.pick("സൂചനകൾ", "flags")) } } }
                            }
                        }
                    }
                }
            }

            @if !data.observations.is_empty() {
                section.sec {
                    h2.h { (t.changes) }
                    div.scroll tabindex="0" role="region" aria-label=(lang.pick("പട്ടിക: വശങ്ങളിലേക്ക് നീക്കാം", "Table, scrolls sideways")) {
                        table {
                            thead { tr { th scope="col" { (t.retrieved) } th scope="col" { (lang.pick("വിവരം", "Field")) } th scope="col" { (lang.pick("മുൻപ് → ഇപ്പോൾ", "Before → after")) } } }
                            tbody {
                                @for o in &data.observations {
                                    tr {
                                        td { (Date::parse_iso(&o.observed_on).map(Date::to_dmy).unwrap_or_default()) }
                                        td.code { (o.field) }
                                        td lang="en" { (o.old_value.as_deref().unwrap_or("—")) " → " (o.new_value.as_deref().unwrap_or("—")) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let description = match (project.approved, project.paid()) {
        (Some(approved), Some(released)) => match lang {
            Lang::Ml => format!("അനുവദിച്ചത് {}, നൽകിയത് {}. ഉറവിടം: കിഫ്ബി.", inr_short(approved, lang), inr_short(released, lang)),
            Lang::En => format!("{} approved, {} paid. Source: KIIFB.", inr_short(approved, lang), inr_short(released, lang)),
        },
        _ => t.list_intro.to_string(),
    };
    layout(
        &Page {
            lang,
            title: &name,
            description: &description,
            path: &path,
            origin,
            nav: Nav::None,
            last_checked: row.last_checked.as_deref(),
            head: html! {},
            image: Some(format!("/og/f/{}.png", encode_segment(&project.reference))),
        },
        body,
    )
}

/// One work: what was approved for it and what has been paid.
fn work(lang: Lang, w: &FundedWork, package: Option<&str>) -> Markup {
    html! {
        article.work {
            div.work-h {
                h3 lang="en" { (display_title(&w.name)) }
                @if let Some(status) = &w.status { span.badge { (status_label(lang, status)) } }
            }
            (paid_line(lang, w))
            @if let Some(code) = package {
                p.small {
                    a href=(format!("{}/p/{}", lang.prefix(), encode_segment(code))) {
                        (icon(icons::MAP_PIN)) " " (lang.pick("ഭൂപട ഡാഷ്ബോർഡിലെ വിവരങ്ങൾ", "Its record on the map dashboard")) " · " span.code { (code) }
                    }
                }
            }
        }
    }
}

/// "₹9.98 crore paid of ₹10.16 crore approved", with a bar.
fn paid_line(lang: Lang, w: &FundedWork) -> Markup {
    html! {
        div.pair {
            div {
                @match w.paid {
                    Some(paid) => { b { (inr_short(paid, lang)) } span { (lang.pick("നൽകിയത്", "paid")) } },
                    None => { b.none { (dash(lang)) } span { (lang.pick("നൽകിയത്", "paid")) } },
                }
                @if let (Some(paid), Some(approved)) = (w.paid, w.approved.filter(|a| *a > 0)) {
                    (meter(paid as f64 / approved as f64, if w.paid_exceeds_approved() { "fl" } else { "" }))
                }
            }
            div {
                @match w.approved {
                    Some(approved) => { b { (inr_short(approved, lang)) } span { (lang.pick("അനുവദിച്ചത്", "approved")) } },
                    None => { b.none { (dash(lang)) } span { (lang.pick("അനുവദിച്ചത്", "approved")) } },
                }
            }
        }
    }
}

/// Where KIIFB's own figures for a project do not agree with each other, say so plainly.
fn source_notes(lang: Lang, project: &FundedProject) -> Markup {
    let paid = project.works_paid();
    let works_approved = project.works_approved();
    html! {
        @if let (Some(multiple), Some(listed)) = (project.listed_multiple(), project.released) {
            p.note.sec {
                @match lang {
                    Lang::Ml => {
                        "കിഫ്ബിയുടെ പട്ടികയിൽ ഈ പദ്ധതിക്ക് “നൽകിയ തുക” " (inr(listed)) " എന്നാണ്. അത് താഴെയുള്ള പ്രവൃത്തികൾക്ക് നൽകിയ ആകെ തുകയുടെ കൃത്യം " (multiple)
                        " മടങ്ങാണ്; പല ജില്ലകളിലായുള്ള പദ്ധതികളിൽ പട്ടിക ഒരേ തുക ആവർത്തിച്ച് കൂട്ടുന്നു. അതിനാൽ ഞങ്ങൾ പ്രവൃത്തികളുടെ ആകെത്തുകയാണ് കാണിക്കുന്നത്: " (inr(paid)) "."
                    },
                    Lang::En => {
                        "KIIFB's list gives “payment released” for this project as " (inr(listed)) ". That is exactly " (multiple)
                        " times what the works below add up to; for projects filed under several districts the list counts the same payments more than once. So we show the total of the works: " (inr(paid)) "."
                    },
                }
            }
        }
        @if let Some(approved) = project.approved.filter(|a| *a > 0 && works_approved as f64 > *a as f64 * 1.01) {
            p.note.sec {
                @match lang {
                    Lang::Ml => {
                        "കിഫ്ബിയുടെ കണക്കുകൾ ഇവിടെ പൊരുത്തപ്പെടുന്നില്ല: പദ്ധതിക്ക് അനുവദിച്ചത് " (inr(approved)) " എന്ന് പട്ടികയിൽ; പ്രവൃത്തികൾക്ക് അനുവദിച്ചതിന്റെ ആകെ " (inr(works_approved)) ". രണ്ടും ഉറവിടത്തിൽ ഉള്ളതുപോലെ കാണിക്കുന്നു."
                    },
                    Lang::En => {
                        "KIIFB's figures do not agree here: its list says " (inr(approved)) " was approved for the project, while the approvals of the works add up to " (inr(works_approved)) ". Both are shown as the source states them."
                    },
                }
            }
        }
    }
}

/// The section on a map package's page: the status-page project it belongs to.
pub fn project_section(lang: Lang, link: &FundingLink, package_count: u32) -> Markup {
    let Ok(project) = serde_json::from_str::<FundedProject>(&link.record_json) else { return html! {} };
    let own: Vec<usize> = link.own_works.as_deref().unwrap_or("").split(',').filter_map(|n| n.parse().ok()).collect();
    let checked = link.detail_checked_at.as_deref().and_then(Date::from_utc_timestamp_ist).map(Date::to_dmy);
    let href = format!("{}/f/{}", lang.prefix(), encode_segment(&project.reference));
    html! {
        section.sec #payments {
            h2.h { (lang.pick("കിഫ്ബി അനുവദിച്ചതും നൽകിയതും", "What KIIFB approved and paid")) }
            div.note {
                p {
                    @match lang {
                        Lang::Ml => {
                            "കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളിൽ ഇത് “" span lang="en" { (display_title(&project.name)) } "” എന്ന പദ്ധതിയുടെ ഭാഗമാണ്."
                            @if package_count > 1 { " താഴെയുള്ള തുകകൾ ആ പദ്ധതിക്ക് മൊത്തമായുള്ളതാണ്; ഈ പാക്കേജിന് മാത്രമല്ല." }
                        },
                        Lang::En => {
                            "On KIIFB's project status page this belongs to “" span lang="en" { (display_title(&project.name)) } "”."
                            @if package_count > 1 { " The figures below are for that whole project, not this package alone." }
                        },
                    }
                }
            }
            dl.kv {
                div {
                    dt { (lang.pick("അനുവദിച്ച തുക", "Approved")) }
                    dd { @match project.approved { Some(a) => { (inr(a)) small { (inr_short(a, lang)) } }, None => span.none { (dash(lang)) } } }
                }
                div {
                    dt { (lang.pick("ഇതുവരെ നൽകിയത്", "Paid so far")) }
                    dd {
                        @match project.paid() {
                            Some(paid) => {
                                (inr(paid))
                                @if let Some(approved) = project.approved.filter(|a| *a > 0 && paid <= *a) {
                                    small { (share_of_approved(lang, paid, approved)) }
                                    (meter(paid as f64 / approved as f64, ""))
                                } @else {
                                    small { (inr_short(paid, lang)) }
                                }
                            },
                            None => span.none { (dash(lang)) },
                        }
                    }
                }
            }
            @for i in &own {
                @if let Some(w) = project.works.get(i.saturating_sub(1)) {
                    article.work {
                        div.work-h {
                            h3 { (lang.pick("ഈ പാക്കേജ്", "This package")) }
                            @if let Some(status) = &w.status { span.badge { (status_label(lang, status)) } }
                        }
                        (paid_line(lang, w))
                    }
                }
            }
            div.actions {
                a.btn.ghost.sm href=(href) {
                    @match (lang, project.works.len()) {
                        (Lang::Ml, 0 | 1) => "കിഫ്ബിയുടെ രേഖ കാണുക",
                        (Lang::En, 0 | 1) => "See KIIFB's record",
                        (Lang::Ml, n) => { "എല്ലാ " (n) " പ്രവൃത്തികളും കാണുക" },
                        (Lang::En, n) => { "See all " (n) " works" },
                    }
                    (icon(icons::ARROW_RIGHT))
                }
            }
            p.small.muted {
                (lang.pick("ഉറവിടം: കിഫ്ബി പ്രോജക്ട് സ്റ്റാറ്റസ് (kiifb.org).", "Source: KIIFB project status (kiifb.org)."))
                @if let Some(date) = checked { " " (lang.pick("പരിശോധിച്ചത്", "Checked")) " " (date) "." }
                " "
                a href=(format!("{}/methodology#join", lang.prefix())) { (lang.pick("ഈ രണ്ട് ഉറവിടങ്ങൾ എങ്ങനെ ചേർത്തു?", "How we joined the two sources")) }
            }
        }
    }
}

/// "86.1% of the amount approved", in each language's word order.
fn share_of_approved(lang: Lang, released: i64, approved: i64) -> String {
    let share = pct(released as f64 / approved as f64 * 100.0);
    match lang {
        Lang::Ml => format!("അനുവദിച്ച തുകയുടെ {share}"),
        Lang::En => format!("{share} of the amount approved"),
    }
}

fn basis_note(lang: Lang, basis: Option<Basis>) -> &'static str {
    match basis {
        Some(Basis::FiguresAndName) => lang.pick(
            "രണ്ട് കിഫ്ബി താളുകൾക്കും പൊതുവായ തിരിച്ചറിയൽ നമ്പറില്ല. അനുവദിച്ച തുകയും വകുപ്പും ഒന്നായതിനാലും പേരുകൾ യോജിക്കുന്നതിനാലും ഞങ്ങൾ ഇവ ചേർത്തു.",
            "KIIFB's two pages share no identifier. We joined these because the approved amount and department are the same and the names agree.",
        ),
        _ => lang.pick(
            "രണ്ട് കിഫ്ബി താളുകൾക്കും പൊതുവായ തിരിച്ചറിയൽ നമ്പറില്ല. അനുവദിച്ച തുകയും വകുപ്പും നിർവഹണ സ്ഥാപനവും ഒന്നായതിനാൽ ഞങ്ങൾ ഇവ ചേർത്തു.",
            "KIIFB's two pages share no identifier. We joined these because the approved amount, department and implementing agency are the same.",
        ),
    }
}

fn status_label(lang: Lang, raw: &str) -> String {
    let label = match raw.to_lowercase().replace(' ', "").as_str() {
        "projectapproved" => lang.pick("അനുമതി ലഭിച്ചു", "Approved"),
        "projectunderevaluation" => lang.pick("പരിശോധനയിൽ", "Under evaluation"),
        "workawarded" => lang.pick("കരാർ നൽകി", "Work awarded"),
        "workstarted" => lang.pick("പണി തുടങ്ങി", "Work started"),
        "workcompleted" => lang.pick("പൂർത്തിയായി", "Completed"),
        _ => return raw.to_string(),
    };
    label.to_string()
}

fn sort_label(lang: Lang, sort: FundingSort) -> &'static str {
    match sort {
        FundingSort::Approved => lang.pick("കൂടുതൽ അനുവദിച്ചവ", "Largest approval"),
        FundingSort::Released => lang.pick("കൂടുതൽ നൽകിയവ", "Most paid"),
        FundingSort::Balance => lang.pick("നൽകാൻ ബാക്കിയുള്ളവ", "Most still to pay"),
        FundingSort::Name => lang.pick("പേര് (A–Z)", "Name (A–Z)"),
    }
}
