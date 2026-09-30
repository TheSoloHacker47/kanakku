use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::{inr, inr_short, pct};
use kanakku_core::gaps;
use kanakku_core::i18n::{flag_explain, flag_label, flag_rule, Lang};
use kanakku_core::model::{Headline, Project, Work};
use kanakku_core::{kiifb, Date};
use maud::{html, Markup};

use super::{en, layout, Nav, Page};
use crate::db::ProjectPage;
use crate::http::encode_segment;

pub fn render(lang: Lang, project: &Project, data: &ProjectPage) -> String {
    let t = lang.t();
    let row = &data.row;
    let headline = project.headline();
    let open_flags = data.flags.iter().filter(|f| f.status == "open").count();
    let gaps = gaps::find(project);
    let retrieved = Date::from_utc_timestamp_ist(&row.fetched_at).map(Date::to_dmy);
    let first_seen = Date::parse_iso(&row.first_seen_on).map(Date::to_dmy);

    let body = html! {
        a.back href=(format!("{}/", lang.prefix())) { "← " (t.back_to_list) }
        p.code { (project.code) }
        h1 lang="en" { (project.title) }

        @if let Some(since) = row.missing_since.as_deref().and_then(Date::parse_iso) {
            p.notice {
                @match lang {
                    Lang::Ml => { (since.to_dmy()) " മുതൽ ഈ പദ്ധതി കിഫ്ബി ഡാഷ്ബോർഡിൽ കാണുന്നില്ല. താഴെയുള്ളത് അവസാനം രേഖപ്പെടുത്തിയ വിവരങ്ങളാണ്." },
                    Lang::En => { "This project has not appeared on the KIIFB dashboard since " (since.to_dmy()) ". The figures below are the last ones recorded." },
                }
            }
        }

        @if let Some((amount, kind)) = headline {
            p.figure {
                b { (inr_short(amount, lang)) }
                span.small {
                    @match kind {
                        Headline::Estimate => (t.estimated_amount),
                        Headline::Spent => (lang.pick("ഈ പാക്കേജിന് രേഖപ്പെടുത്തിയ ചെലവ്", "Expenditure reported for this package")),
                        Headline::WorksTotal => (t.works_total),
                    }
                    " · " (inr(amount)) " · "
                    a href="#source" { (t.sources) }
                }
            }
        }

        section aria-labelledby="flags" {
            h2 #flags { (t.flags) @if open_flags > 0 { " (" (open_flags) ")" } }
            @if data.flags.is_empty() {
                p.intro { (t.no_flags) }
            } @else {
                ul.flags {
                    @for flag in &data.flags {
                        @if let Some(kind) = FlagKind::parse(&flag.kind) {
                            @let value: serde_json::Value = serde_json::from_str(&flag.value_json).unwrap_or_default();
                            li.cleared[flag.status != "open"] {
                                h3 { (flag_label(lang, kind)) }
                                @if let Some(work) = &flag.work_ref {
                                    p.small { (t.work) ": " (en(work)) }
                                }
                                p { (flag_explain(lang, kind, &value)) }
                                p.small {
                                    (flag_rule(lang, kind)) " "
                                    a href=(format!("{}/methodology#rules", lang.prefix())) { (t.rule_version) " " (flag.rule_version) }
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
                                            Lang::En => { "Raised on " (raised.to_dmy()) },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        section aria-labelledby="money" {
            h2 #money { (t.money_trail) }
            dl.rows {
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
                                @if !project.estimate_is_shared() { (share(lang, spent, project.estimated_amount)) }
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
                                Some(spent) => { (inr(spent)) (share(lang, spent, project.estimated_amount)) },
                                None => span.none { (t.not_reported) },
                            }
                        }
                    }
                }
                (text_row(t.status, project.status.as_deref(), true))
            }
        }

        @if !project.works.is_empty() {
            section aria-labelledby="works" {
                h2 #works { (t.works) " (" (project.works.len()) ")" }
                @for (i, w) in project.works.iter().enumerate() {
                    (work(lang, w, i))
                }
            }
        }

        section aria-labelledby="about" {
            h2 #about { (lang.pick("വിശദാംശങ്ങൾ", "Details")) }
            dl.rows {
                div { dt { (t.code) } dd.code { (project.code) } }
                (text_row(t.department, project.department.as_deref(), false))
                (text_row(t.executing_agency, project.executing_agency.as_deref(), false))
                @for c in &project.constituencies {
                    div {
                        dt { (t.constituency) }
                        dd {
                            @match (lang, &c.name_ml) {
                                (Lang::Ml, Some(name)) => (name),
                                _ => (en(&c.name)),
                            }
                            @let mla = match lang {
                                Lang::Ml => c.mla_name_ml.as_deref().or(c.mla_name.as_deref()),
                                Lang::En => c.mla_name.as_deref(),
                            };
                            @if let Some(mla) = mla {
                                small { (t.mla) ": " (mla) }
                            }
                        }
                    }
                }
            }
        }

        section aria-labelledby="gaps" {
            h2 #gaps { (t.gaps) }
            @if gaps.is_empty() {
                p.intro { (t.no_gaps) }
            } @else {
                p.intro { (t.gaps_intro) }
                ul.list {
                    @for gap in &gaps {
                        li {
                            (gaps::label(lang, gap.kind))
                            @if let Some(work) = &gap.work_ref { span.small { " · " (en(work)) } }
                        }
                    }
                }
            }
        }

        @if !project.sites.is_empty() || project.works.iter().any(|w| w.lat.is_some()) {
            section aria-labelledby="location" {
                h2 #location { (t.location) }
                ul.list {
                    @for site in &project.sites {
                        li { (coordinate_link(lang, site.lat, site.lng, None)) }
                    }
                    @for (i, w) in project.works.iter().enumerate() {
                        @if let (Some(lat), Some(lng)) = (w.lat, w.lng) {
                            li { (coordinate_link(lang, lat, lng, Some(&w.reference(i)))) }
                        }
                    }
                }
                p.small { (t.map_intro) }
            }
        }

        section aria-labelledby="source" {
            h2 #source { (t.sources) }
            dl.rows {
                div {
                    dt { (t.sources) }
                    dd { a href=(kiifb::SOURCE_URL) rel="noopener" { (t.source_kiifb) } small { "gis.kiifb.org" } }
                }
                (text_row(t.retrieved, retrieved.as_deref(), false))
                (text_row(t.first_seen, first_seen.as_deref(), false))
                div {
                    dt { "SHA-256" }
                    dd.code { (&row.sha256[..row.sha256.len().min(16)]) "…" }
                }
            }
            p { a href=(format!("/snapshot/{}", row.snapshot_id)) rel="nofollow" { (t.download_snapshot) } }
            p.small {
                a href=(format!("/api/v1/projects/{}", encode_segment(&project.code))) { "JSON" }
            }
        }

        @if !data.observations.is_empty() {
            section aria-labelledby="changes" {
                h2 #changes { (t.changes) }
                table {
                    thead { tr { th { (t.retrieved) } th { (lang.pick("വിവരം", "Field")) } th { (lang.pick("മുൻപ് → ഇപ്പോൾ", "Before → after")) } } }
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
    };

    let description = format!("{} · {}", project.code, project.title);
    let path = format!("/p/{}", encode_segment(&project.code));
    layout(
        &Page {
            lang,
            title: &project.title,
            description: &description,
            path: &path,
            nav: Nav::None,
            last_checked: row.last_checked.as_deref(),
            head: html! {},
        },
        body,
    )
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
fn share(lang: Lang, spent: i64, estimate: Option<i64>) -> Markup {
    html! {
        @if let Some(estimate) = estimate.filter(|e| *e > 0) {
            small { (pct(spent as f64 / estimate as f64 * 100.0)) " " (lang.t().spent_share) }
        }
    }
}

fn work(lang: Lang, w: &Work, index: usize) -> Markup {
    let t = lang.t();
    html! {
        article.work {
            h3 {
                @match &w.road_name {
                    Some(name) => (en(name)),
                    None => { (t.work) " #" (index + 1) },
                }
            }
            dl.rows {
                (text_row(t.status, w.status.as_deref(), true))
                (text_row(t.contractor, w.contractor.as_deref(), true))
                (text_row(lang.pick("നിർവഹണ സ്ഥാപനം (SPV)", "Implementing agency (SPV)"), w.spv.as_deref(), false))
                (text_row(t.scheduled_start, w.scheduled_start.map(Date::to_dmy).as_deref(), true))
                (text_row(t.scheduled_end, w.scheduled_end.map(Date::to_dmy).as_deref(), true))
                (progress_row(t.physical_progress, w.physical_pct))
                (progress_row(t.financial_progress, w.financial_pct))
                @if let Some(note) = &w.progress_note {
                    div.long { dt { (t.progress_note) } dd lang="en" { (note) } }
                }
                (amount_row(lang, t.as_amount, w.as_amount, false))
                (amount_row(lang, t.fs_amount, w.fs_amount, false))
                (amount_row(lang, t.ts_amount, w.ts_amount, false))
                (amount_row(lang, t.tender_amount, w.tender_amount, false))
                (amount_row(lang, t.loa_amount, w.loa_amount, false))
                (amount_row(lang, t.contract_amount, w.contract_amount, true))
                (amount_row(lang, t.paid_amount, w.paid_amount, true))
                (amount_row(lang, t.paid_contractor, w.paid_contractor, false))
            }
        }
    }
}

/// A label and a text value. Rows marked `always` show "Not published" when the value is missing;
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

fn progress_row(label: &str, value: Option<f64>) -> Markup {
    html! {
        @if let Some(v) = value {
            div {
                dt { (label) }
                dd {
                    (pct(v))
                    progress max="100" value=(format!("{:.1}", v.clamp(0.0, 100.0))) aria-label=(label) {}
                }
            }
        }
    }
}

fn coordinate_link(lang: Lang, lat: f64, lng: f64, label: Option<&str>) -> Markup {
    html! {
        @if let Some(label) = label { (en(label)) " · " }
        span.code { (format!("{lat:.5}, {lng:.5}")) }
        " · "
        a href=(format!("https://www.openstreetmap.org/?mlat={lat:.5}&mlon={lng:.5}#map=16/{lat:.5}/{lng:.5}")) rel="noopener" {
            (lang.t().open_in_maps)
        }
    }
}
