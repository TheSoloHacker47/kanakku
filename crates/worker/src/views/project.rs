//! One project: its figures, its stage, its flags, what is missing, and where it all came from.

use kanakku_core::entity::{agency_display, agency_key, contractor_display, contractor_key};
use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::{inr, inr_short, pct};
use kanakku_core::gaps;
use kanakku_core::i18n::{flag_explain, flag_label, flag_rule, Lang};
use kanakku_core::model::{Headline, Project, Work};
use kanakku_core::names::{constituency_label, department_label, district_label};
use kanakku_core::stage::Stage;
use kanakku_core::title::display_title;
use kanakku_core::{kiifb, Date};
use maud::{html, Markup};
use worker::url::form_urlencoded::byte_serialize;

use super::{big_amount, department_icon, dot_map, en, funding, icon, layout, meter, on_dot_map, Nav, Page};
use crate::db::ProjectPage;
use crate::http::encode_segment;
use crate::icons;

pub fn render(lang: Lang, origin: &str, project: &Project, data: &ProjectPage, today: Date) -> String {
    let t = lang.t();
    let p = lang.prefix();
    let row = &data.row;
    let title = display_title(&project.title);
    let path = format!("/p/{}", encode_segment(&project.code));
    let url = format!("{origin}{p}{path}");
    let open_flags: Vec<_> = data.flags.iter().filter(|f| f.status == "open").collect();
    let gaps = gaps::find(project);
    let retrieved = Date::from_utc_timestamp_ist(&row.fetched_at).map(Date::to_dmy);
    let first_seen = Date::parse_iso(&row.first_seen_on).map(Date::to_dmy);
    let stage = project.status.as_deref().and_then(Stage::from_status);
    let here: Vec<(f64, f64)> = project
        .sites
        .iter()
        .map(|s| (s.lat, s.lng))
        .chain(project.works.iter().filter_map(|w| Some((w.lat?, w.lng?))))
        .collect();
    let on_map = here.iter().any(|(lat, lng)| on_dot_map(&project.district, *lat, *lng));
    let share = format!("https://wa.me/?text={}", byte_serialize(format!("{title} — {url}").as_bytes()).collect::<String>());

    let body = html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{p}/")) { (t.site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    a href=(format!("{p}/projects")) { (t.nav_projects) }
                    @if !project.district.is_empty() {
                        (icon(icons::CHEVRON_RIGHT))
                        a href=(format!("{p}/d/{}", project.district.to_lowercase())) { (district_label(lang, &project.district)) }
                    }
                    @if let Some(department) = &project.department {
                        (icon(icons::CHEVRON_RIGHT))
                        a href=(format!("{p}/projects?dept={}", byte_serialize(department.as_bytes()).collect::<String>())) { (department_label(lang, department)) }
                    }
                }
                div.meta {
                    span.ico { (icon(department_icon(project.department.as_deref()))) }
                    span.code { (project.code) }
                    @for c in &project.constituencies {
                        a href=(format!("{p}/projects?lac={}", byte_serialize(c.name.as_bytes()).collect::<String>())) {
                            (icon(icons::MAP_PIN)) " " (constituency_label(lang, &c.name))
                        }
                    }
                }
                h1 lang="en" { (title) }
                @if !open_flags.is_empty() {
                    div.tags {
                        @for flag in &open_flags {
                            @if let Some(kind) = FlagKind::parse(&flag.kind) {
                                a.badge.flag href="#flags" { (icon(icons::FLAG)) (flag_label(lang, kind)) }
                            }
                        }
                    }
                }
            }
        }

        div.wrap {
            div.panels {
                (figure_panel(lang, project))
                (stage_panel(lang, project, stage))
                div.panel {
                    h2 { (t.location) }
                    @if on_map {
                        a href=(format!("{p}/map#{}", encode_segment(&project.code))) aria-label=(t.open_in_maps) {
                            (dot_map(&project.district, &[], &here, lang.pick("ഈ പദ്ധതിയുടെ സ്ഥാനം", "Where this project is"), true))
                        }
                    } @else if here.is_empty() {
                        p.muted { (t.not_reported) }
                    } @else {
                        p.muted { (lang.pick("കിഫ്ബി രേഖപ്പെടുത്തിയ സ്ഥാനം ഈ ഭൂപടത്തിന് പുറത്താണ്.", "The location KIIFB records lies outside this map.")) }
                    }
                }
            }

            @if let Some(since) = row.missing_since.as_deref().and_then(Date::parse_iso) {
                p.note.warn.sec {
                    @match lang {
                        Lang::Ml => { (since.to_dmy()) " മുതൽ ഈ പദ്ധതി കിഫ്ബി ഡാഷ്ബോർഡിൽ കാണുന്നില്ല. താഴെയുള്ളത് അവസാനം രേഖപ്പെടുത്തിയ വിവരങ്ങളാണ്." },
                        Lang::En => { "This project has not appeared on the KIIFB dashboard since " (since.to_dmy()) ". The figures below are the last ones recorded." },
                    }
                }
            }

            @if !data.flags.is_empty() {
                section.sec #flags {
                    h2.h { (t.flags) @if !open_flags.is_empty() { " (" (open_flags.len()) ")" } }
                    @for flag in &data.flags {
                        @if let Some(kind) = FlagKind::parse(&flag.kind) {
                            @let value: serde_json::Value = serde_json::from_str(&flag.value_json).unwrap_or_default();
                            div.flagbox.off[flag.status != "open"] {
                                (icon(icons::FLAG))
                                h3 {
                                    (flag_label(lang, kind))
                                    @if let Some(work) = &flag.work_ref { " · " (en(work)) }
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
                                    } @else if let Some(raised) = Date::parse_iso(&flag.created_on) {
                                        " · "
                                        @match lang {
                                            Lang::Ml => { (raised.to_dmy()) " മുതൽ" },
                                            Lang::En => { "Since " (raised.to_dmy()) },
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
                    h2.h { (t.works) " (" (project.works.len()) ")" }
                    @for (i, w) in project.works.iter().enumerate() { (work(lang, w, i, today)) }
                }
            }

            div.cols.sec {
                section {
                    h2.h { (t.money_trail) }
                    dl.kv {
                        div {
                            dt { (t.estimated_amount) }
                            dd {
                                @match project.estimated_amount {
                                    Some(a) => {
                                        (inr(a))
                                        small { @if project.estimate_is_shared() { (shared_note(lang, project)) } @else { (inr_short(a, lang)) } }
                                    },
                                    None => span.none { (t.not_reported) },
                                }
                            }
                        }
                        div {
                            dt { (t.expenditure) @if project.estimate_is_shared() { " · " (lang.pick("ഈ പാക്കേജ്", "this package")) } }
                            dd {
                                @match project.expenditure {
                                    Some(spent) => {
                                        (inr(spent))
                                        @if !project.estimate_is_shared() { (share_of(lang, spent, project.estimated_amount)) }
                                    },
                                    None => span.none { (t.not_reported) },
                                }
                            }
                        }
                        @if project.estimate_is_shared() {
                            div {
                                dt {
                                    @match lang {
                                        Lang::Ml => { (t.expenditure) " · എല്ലാ " (project.estimate_shared_by) " പാക്കേജുകളും" },
                                        Lang::En => { (t.expenditure) " · all " (project.estimate_shared_by) " packages" },
                                    }
                                }
                                dd {
                                    @match project.group_expenditure {
                                        Some(spent) => { (inr(spent)) (share_of(lang, spent, project.estimated_amount)) },
                                        None => span.none { (t.not_reported) },
                                    }
                                }
                            }
                        }
                        @if let Some(total) = project.works_amount().filter(|_| project.estimated_amount.is_none()) {
                            div { dt { (t.works_total) } dd { (inr(total)) small { (inr_short(total, lang)) } } }
                        }
                    }
                }
                section {
                    h2.h { (lang.pick("വിശദാംശങ്ങൾ", "Details")) }
                    dl.kv {
                        div { dt { (t.code) } dd.code { (project.code) } }
                        @if let Some(department) = &project.department {
                            div { dt { (t.department) } dd { (department_label(lang, department)) } }
                        }
                        @if let Some(agency) = &project.executing_agency {
                            div { dt { (t.executing_agency) } dd lang="en" { a href=(format!("{p}/a/{}", agency_key(agency))) { (agency_display(agency)) } } }
                        }
                        @for c in &project.constituencies {
                            div {
                                dt { (t.constituency) }
                                dd {
                                    (constituency_label(lang, &c.name))
                                    @let mla = match lang {
                                        Lang::Ml => c.mla_name_ml.as_deref().or(c.mla_name.as_deref()),
                                        Lang::En => c.mla_name.as_deref(),
                                    };
                                    @if let Some(mla) = mla { small { (t.mla) ": " (mla) } }
                                }
                            }
                        }
                        @if title != project.title {
                            div.long { dt { (lang.pick("കിഫ്ബി പ്രസിദ്ധീകരിച്ച പേര്", "Name as KIIFB publishes it")) } dd lang="en" { (project.title) } }
                        }
                    }
                }
            }

            @if let Some(link) = &data.funding { (funding::project_section(lang, link, project.estimate_shared_by)) }

            div.cols.sec {
                section {
                    h2.h { (t.gaps) }
                    @if gaps.is_empty() {
                        p.muted { (t.no_gaps) }
                    } @else {
                        div.note {
                            p { (t.gaps_intro) }
                            ul {
                                @for gap in &gaps {
                                    li {
                                        (icon(icons::CIRCLE_HELP)) " " b { (gaps::label(lang, gap.kind)) }
                                        @if let Some(work) = &gap.work_ref { span.small { " · " (en(work)) } }
                                    }
                                }
                            }
                        }
                    }
                }
                section #source {
                    h2.h { (t.sources) }
                    dl.kv {
                        div { dt { (t.sources) } dd { (t.source_kiifb) small { "gis.kiifb.org" } } }
                        @if let Some(date) = &retrieved { div { dt { (t.retrieved) } dd { (date) } } }
                        @if let Some(date) = &first_seen { div { dt { (t.first_seen) } dd { (date) } } }
                        div { dt { "SHA-256" } dd.code { (&row.sha256[..row.sha256.len().min(16)]) "…" } }
                    }
                    div.actions {
                        a.btn.ghost.sm href=(format!("/snapshot/{}", row.snapshot_id)) rel="nofollow" { (icon(icons::DOWNLOAD)) (lang.pick("സൂക്ഷിച്ച പകർപ്പ്", "Stored copy")) }
                        a.btn.ghost.sm href=(kiifb::SOURCE_URL) rel="noopener" { (lang.pick("കിഫ്ബി ഡാഷ്ബോർഡ്", "KIIFB dashboard")) (icon(icons::ARROW_UP_RIGHT)) }
                        a.btn.ghost.sm href=(format!("/api/v1/projects/{}", encode_segment(&project.code))) { (icon(icons::DATABASE)) "JSON" }
                        a.btn.sm href=(share) rel="noopener" { (icon(icons::SHARE_2)) (lang.pick("വാട്സ്ആപ്പിൽ പങ്കിടുക", "Share on WhatsApp")) }
                    }
                }
            }

            @if !data.siblings.is_empty() {
                section.sec {
                    h2.h {
                        @match lang {
                            Lang::Ml => { "ഇതേ ഉപപദ്ധതിയിലെ മറ്റ് പാക്കേജുകൾ" },
                            Lang::En => { "Other packages under the same sub-project" },
                        }
                    }
                    ul.list {
                        @for s in &data.siblings {
                            li {
                                a href=(format!("{p}/p/{}", encode_segment(&s.code))) lang="en" { (display_title(&s.title_en)) }
                                " "
                                span.small.muted {
                                    @match s.expenditure {
                                        Some(spent) => { (inr_short(spent, lang)) " " (lang.pick("ചെലവ്", "spent")) },
                                        None => (t.not_reported),
                                    }
                                }
                                @if s.flag_count > 0 { " " span.badge.flag { (icon(icons::FLAG)) (s.flag_count) } }
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

    let description = match project.headline() {
        Some((amount, _)) => format!("{} · {} · {}", project.code, inr_short(amount, lang), t.list_intro),
        None => format!("{} · {}", project.code, t.list_intro),
    };
    layout(
        &Page {
            lang,
            title: &title,
            description: &description,
            path: &path,
            origin,
            nav: Nav::None,
            last_checked: row.last_checked.as_deref(),
            head: html! {},
            image: Some(format!("/og/p/{}.png", encode_segment(&project.code))),
        },
        body,
    )
}

/// The one big figure, on green.
fn figure_panel(lang: Lang, project: &Project) -> Markup {
    let t = lang.t();
    html! {
        div.panel.g {
            @match project.headline() {
                Some((amount, kind)) => {
                    h2 {
                        @match kind {
                            Headline::Estimate => (t.estimated_amount),
                            Headline::Spent => (lang.pick("ഈ പാക്കേജിന് ഇതുവരെ ചെലവ്", "Spent on this package so far")),
                            Headline::WorksTotal => (t.works_total),
                        }
                    }
                    p.num { (big_amount(amount, lang)) }
                    p.small {
                        (inr(amount))
                        @if kind == Headline::Estimate {
                            @if let Some(spent) = project.expenditure {
                                " · " (inr_short(spent, lang)) " " (lang.pick("ചെലവ്", "spent"))
                            }
                        }
                    }
                    @if kind == Headline::Estimate {
                        @if let (Some(spent), Some(estimate)) = (project.expenditure, project.estimated_amount.filter(|e| *e > 0)) {
                            (meter(spent as f64 / estimate as f64, "k"))
                        }
                    }
                },
                None => {
                    h2 { (t.estimated_amount) }
                    p.num { "—" }
                    p.small { (lang.pick("ഈ പാക്കേജിന് സ്വന്തമായ തുക പ്രസിദ്ധീകരിച്ചിട്ടില്ല.", "No figure is published for this package alone.")) }
                },
            }
        }
    }
}

/// Where the project stands, as a track of plain stages.
fn stage_panel(lang: Lang, project: &Project, stage: Option<Stage>) -> Markup {
    let t = lang.t();
    html! {
        div.panel {
            h2 { (lang.pick("ഘട്ടം", "Stage")) }
            @match stage {
                Some(Stage::Returned) => p { span.badge { (Stage::Returned.label(lang)) } },
                Some(current) => {
                    ol.track {
                        @for step in Stage::TRACK {
                            li.done[step < current].now[step == current] aria-current=[(step == current).then_some("step")] { (step.label(lang)) }
                        }
                    }
                },
                None => {
                    @if project.works.is_empty() {
                        p.muted { (t.not_reported) }
                    } @else {
                        // Works-only projects have no project status; show what the works say.
                        p.chips {
                            @for (status, count) in work_statuses(project) {
                                span.badge { (work_status(lang, &status)) @if project.works.len() > 1 { " × " (count) } }
                            }
                        }
                        p.small.muted { (lang.pick("പ്രവൃത്തികളുടെ നില, കിഫ്ബി രേഖപ്പെടുത്തിയത്.", "Status of the works, as KIIFB records it.")) }
                    }
                },
            }
            @if let Some(status) = &project.status {
                p.small.muted {
                    (lang.pick("കിഫ്ബി നില", "KIIFB status")) ": " (en(status)) " · "
                    a href=(format!("{}/methodology#stages", lang.prefix())) { (lang.pick("ഘട്ടങ്ങൾ എങ്ങനെ?", "How we group")) }
                }
            }
        }
    }
}

fn work(lang: Lang, w: &Work, index: usize, today: Date) -> Markup {
    let t = lang.t();
    html! {
        article.work {
            div.work-h {
                h3 {
                    @match &w.road_name {
                        Some(name) => (en(name)),
                        None => { (t.work) " " (index + 1) },
                    }
                }
                @if let Some(status) = &w.status { span.badge { (work_status(lang, status)) } }
            }
            @if w.physical_pct.is_some() || w.financial_pct.is_some() {
                div.pair {
                    (progress(t.physical_progress, w.physical_pct, "k"))
                    (progress(t.financial_progress, w.financial_pct, ""))
                }
            }
            (schedule(lang, w, today))
            dl.kv {
                div {
                    dt { (t.contractor) }
                    dd {
                        @match w.contractor.as_deref() {
                            Some(name) => a href=(format!("{}/c/{}", lang.prefix(), contractor_key(name))) lang="en" { (contractor_display(name)) },
                            None => span.none { "—" },
                        }
                    }
                }
                (text_row(lang.pick("നിർവഹണ സ്ഥാപനം (SPV)", "Implementing agency (SPV)"), w.spv.as_deref(), false))
                (amount_row(lang, t.contract_amount, w.contract_amount, true))
                (amount_row(lang, t.paid_amount, w.paid_amount, true))
                (amount_row(lang, t.paid_contractor, w.paid_contractor, false))
                (amount_row(lang, t.as_amount, w.as_amount, false))
                (amount_row(lang, t.fs_amount, w.fs_amount, false))
                (amount_row(lang, t.ts_amount, w.ts_amount, false))
                (amount_row(lang, t.tender_amount, w.tender_amount, false))
                (amount_row(lang, t.loa_amount, w.loa_amount, false))
                @if let Some(note) = &w.progress_note {
                    div.long { dt { (t.progress_note) } dd lang="en" { (note) } }
                }
            }
        }
    }
}

fn progress(label: &str, value: Option<f64>, class: &str) -> Markup {
    html! {
        div {
            @match value {
                Some(v) => { b { (pct(v)) } span { (label) } (meter(v / 100.0, class)) },
                None => { b.none { "—" } span { (label) } },
            }
        }
    }
}

/// The scheduled span as a line, with today marked and any overrun in the flag colour.
fn schedule(lang: Lang, w: &Work, today: Date) -> Markup {
    let t = lang.t();
    let (Some(start), Some(end)) = (w.scheduled_start, w.scheduled_end) else {
        return html! {
            @if w.scheduled_start.is_some() || w.scheduled_end.is_some() {
                dl.kv {
                    (text_row(t.scheduled_start, w.scheduled_start.map(Date::to_dmy).as_deref(), true))
                    (text_row(t.scheduled_end, w.scheduled_end.map(Date::to_dmy).as_deref(), true))
                }
            }
        };
    };
    let last = if today > end { today } else { end };
    let total = last.days_since(start).max(1) as f64;
    let at = |d: Date| format!("{:.1}%", 1.5 + (d.days_since(start).clamp(0, total as i32) as f64 / total) * 97.0);
    let overdue = today > end && !w.is_completed();
    html! {
        svg.span role="img" aria-label=(format!("{} {} – {} {}", t.scheduled_start, start.to_dmy(), t.scheduled_end, end.to_dmy())) {
            line.base x1="1.5%" x2="98.5%" y1="11" y2="11" {}
            line.plan x1=(at(start)) x2=(at(end)) y1="11" y2="11" {}
            @if overdue { line.late x1=(at(end)) x2=(at(today)) y1="11" y2="11" {} }
            @if today >= start && (overdue || today <= end) { line.today x1=(at(today)) x2=(at(today)) y1="1" y2="21" {} }
        }
        div.ends {
            div { (t.scheduled_start) b { (start.to_dmy()) } }
            div { (t.scheduled_end) b { (end.to_dmy()) } }
        }
    }
}

/// Each distinct work status with how many works carry it, in first-seen order.
fn work_statuses(project: &Project) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for status in project.works.iter().filter_map(|w| w.status.as_deref()) {
        match out.iter_mut().find(|(known, _)| known == status) {
            Some((_, count)) => *count += 1,
            None => out.push((status.to_string(), 1)),
        }
    }
    out
}

fn work_status(lang: Lang, raw: &str) -> String {
    let label = match raw.to_lowercase().replace(' ', "").as_str() {
        "approved" => lang.pick("അംഗീകരിച്ചു", "Approved"),
        "underappraisal" => lang.pick("പരിശോധനയിൽ", "Under appraisal"),
        "inprogress" => lang.pick("പുരോഗമിക്കുന്നു", "In progress"),
        "completed" => lang.pick("പൂർത്തിയായി", "Completed"),
        "deferred" => lang.pick("മാറ്റിവെച്ചു", "Deferred"),
        _ => return raw.to_string(),
    };
    label.to_string()
}

/// "Estimate of sub-project AGR001-01, shared by 4 contract packages".
fn shared_note(lang: Lang, project: &Project) -> Markup {
    let sub = project.sub_project_code.as_deref().unwrap_or("");
    html! {
        @match lang {
            Lang::Ml => { "ഉപപദ്ധതി " span.code { (sub) } "-ന്റെ കണക്കാക്കിയ തുക; " (project.estimate_shared_by) " കരാർ പാക്കേജുകൾക്ക് പൊതുവായത്" },
            Lang::En => { "Estimate of sub-project " span.code { (sub) } ", shared by " (project.estimate_shared_by) " contract packages" },
        }
    }
}

/// "58.4% of the estimated amount", when there is an estimate to compare with.
fn share_of(lang: Lang, spent: i64, estimate: Option<i64>) -> Markup {
    html! {
        @if let Some(estimate) = estimate.filter(|e| *e > 0) {
            small { (pct(spent as f64 / estimate as f64 * 100.0)) " " (lang.t().spent_share) }
            (meter(spent as f64 / estimate as f64, ""))
        }
    }
}

/// A label and a text value. Rows marked `always` show a dash when the value is missing;
/// the others are left out.
fn text_row(label: &str, value: Option<&str>, always: bool) -> Markup {
    html! {
        @if value.is_some() || always {
            div {
                dt { (label) }
                dd {
                    @match value {
                        Some(v) => (en(v)),
                        None => span.none { "—" },
                    }
                }
            }
        }
    }
}

fn amount_row(lang: Lang, label: &str, amount: Option<i64>, always: bool) -> Markup {
    html! {
        @if amount.is_some() || always {
            div {
                dt { (label) }
                dd {
                    @match amount {
                        Some(a) => { (inr(a)) small { (inr_short(a, lang)) } },
                        None => span.none { (lang.t().not_reported) },
                    }
                }
            }
        }
    }
}
