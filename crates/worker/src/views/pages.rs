//! The standing pages: methodology, open data, map and not-found.

use kanakku_core::flags::{FlagKind, RULES_VERSION};
use kanakku_core::i18n::{flag_label, flag_rule, Lang};
use kanakku_core::stage::Stage;
use maud::{html, Markup};

use super::{icon, layout, Nav, Page};
use crate::icons;

fn head(lang: Lang, title: &str, lead: &str) -> Markup {
    html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{}/", lang.prefix())) { (lang.t().site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    span { (title) }
                }
                h1.d2 { (title) }
                p.lead { (lead) }
            }
        }
    }
}

pub fn methodology(lang: Lang, origin: &str, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let lead = lang.pick(
        "സർക്കാർ തന്നെ പ്രസിദ്ധീകരിക്കുന്ന പദ്ധതി വിവരങ്ങൾ ശേഖരിച്ച്, വായിക്കാവുന്ന രൂപത്തിൽ, ഉറവിടത്തോടൊപ്പം കാണിക്കുന്നു. അതു മാത്രം.",
        "We collect the project figures the government itself publishes and show them in a readable form, next to their source. Nothing more.",
    );
    let body = html! {
        (head(lang, t.methodology_title, lead))
        div.wrap {
            div.prose {
                h2 { (lang.pick("തത്വങ്ങൾ", "Principles")) }
                @match lang {
                    Lang::Ml => ul {
                        li { "ഓരോ സംഖ്യയും അതിന്റെ ഉറവിടത്തിലേക്ക് ചൂണ്ടുന്നു. ഞങ്ങൾ ശേഖരിച്ച താളിന്റെ പകർപ്പ് സൂക്ഷിക്കുന്നു; അത് ആർക്കും ഡൗൺലോഡ് ചെയ്യാം." }
                        li { "വസ്തുതകളും സൂചനകളും മാത്രം. “14 മാസം വൈകി” എന്ന് പറയും; കാരണം ഊഹിക്കില്ല, ആരെയും കുറ്റപ്പെടുത്തില്ല." }
                        li { "എല്ലാ കക്ഷികൾക്കും എം.എൽ.എമാർക്കും തദ്ദേശ സ്ഥാപനങ്ങൾക്കും ഒരേ നിയമങ്ങൾ." }
                    },
                    Lang::En => ul {
                        li { "Every number points to its source. We keep a copy of the page we collected it from, and anyone can download that copy." }
                        li { "Facts and flags only. We say “14 months past scheduled completion”; we do not guess at reasons or blame anyone." }
                        li { "The same rules apply to every party, MLA and local body." }
                    },
                }

                h2 #rules { (lang.pick("സൂചനാ നിയമങ്ങൾ", "Flag rules")) " · " (t.rule_version) " " (RULES_VERSION) }
                p { (lang.pick(
                    "ഒരു സൂചന എന്നത് പ്രസിദ്ധീകരിച്ച സംഖ്യകൾ ഒരു പരിധി കടന്നു എന്നതിന്റെ അടയാളം മാത്രമാണ്. അത് തെറ്റ് നടന്നു എന്നതിന്റെ തെളിവല്ല. ഓരോ സൂചനയിലും അതിന് ആധാരമായ സംഖ്യകളും നിയമത്തിന്റെ പതിപ്പും കാണാം.",
                    "A flag only marks that published figures crossed a threshold. It is not evidence of wrongdoing. Each flag shows the figures it rests on and the version of the rule that raised it.",
                )) }
                div.rules {
                    @for kind in FlagKind::ALL {
                        div {
                            a.badge.flag href=(format!("{}/projects?flag={}", lang.prefix(), kind.as_str())) { (icon(icons::FLAG)) (flag_label(lang, kind)) }
                            p { (flag_rule(lang, kind)) }
                        }
                    }
                }

                h2 #stages { (lang.pick("ഘട്ടങ്ങൾ", "Stages")) }
                p { (lang.pick(
                    "കിഫ്ബിയുടെ സ്ഥിതിവിവരങ്ങൾ (“WBS Base One Approved” പോലുള്ളവ) അവരുടെ ആഭ്യന്തര പ്രവർത്തനക്രമത്തിലെ പദങ്ങളാണ്. വായിക്കാൻ എളുപ്പത്തിന് ഞങ്ങൾ അവയെ താഴെയുള്ള ഘട്ടങ്ങളാക്കി തിരിക്കുന്നു. ഈ തരംതിരിവ് ഞങ്ങളുടേതാണ്; കിഫ്ബിയുടെ യഥാർത്ഥ പദം ഓരോ പദ്ധതിയുടെയും താളിൽ കാണിക്കുന്നു.",
                    "KIIFB's status labels, such as “WBS Base One Approved”, are terms from its internal workflow. To make them readable we group them into the stages below. The grouping is ours; each project page also shows KIIFB's original label.",
                )) }
                div.scroll {
                    table {
                        thead { tr { th { (lang.pick("ഘട്ടം", "Stage")) } th { (lang.pick("കിഫ്ബിയുടെ പദങ്ങൾ", "KIIFB labels containing")) } } }
                        tbody {
                            @for stage in Stage::ALL {
                                tr {
                                    td { b { (stage.label(lang)) } }
                                    td lang="en" {
                                        @match stage {
                                            Stage::Preparation => "Project Initiated, Project Created, Design Basis Report, Project Execution Document, Submitted for Confirmation",
                                            Stage::TechnicalSanction => "Technical Sanction",
                                            Stage::Tender => "Tender",
                                            Stage::Contract => "Contract Process",
                                            Stage::WorkPlan => "WBS, Payment milestone",
                                            Stage::Returned => "Return to SPV",
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                h2 { (lang.pick("ഡാറ്റ എവിടെ നിന്ന്", "Where the data comes from")) }
                p { (lang.pick(
                    "കിഫ്ബിയുടെ സംയോജിത ഡാഷ്ബോർഡ് (gis.kiifb.org) ദിവസത്തിൽ ഒരിക്കൽ ഞങ്ങൾ വായിക്കുന്നു. ഉള്ളടക്കം മാറിയിട്ടുണ്ടെങ്കിൽ മാത്രം പുതിയ പകർപ്പ് സൂക്ഷിക്കും; മാറിയ ഓരോ വിവരവും പദ്ധതിയുടെ താളിൽ രേഖപ്പെടുത്തും.",
                    "We read KIIFB's integrated dashboard (gis.kiifb.org) once a day. We store a new copy only when its contents have changed, and every field that changed is recorded on the project's page.",
                )) }

                p { (lang.pick(
                    "കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളും (kiifb.org/prjStatus.jsp) ദിവസവും വായിക്കുന്നു. അതിൽ ഓരോ പദ്ധതിക്കും അനുവദിച്ച തുകയും നൽകിയ തുകയും, ഓരോ പ്രവൃത്തിക്കും അനുവദിച്ചതും നൽകിയതും ഉണ്ട്. തുക മാറിയ പദ്ധതികളുടെ പ്രവൃത്തിപ്പട്ടിക അന്നുതന്നെ വീണ്ടും വായിക്കും; മറ്റുള്ളവ ഊഴമനുസരിച്ച്, ഓരോ രാത്രിയും കുറച്ചെണ്ണം വീതം.",
                    "We also read KIIFB's project status page (kiifb.org/prjStatus.jsp) once a day. It gives, for each project, the amount approved and the amount released, and the same for each work. When a project's figures move we re-read its work table the same night; the rest are re-read in turn, a few each night.",
                )) }

                h2 #join { (lang.pick("രണ്ട് ഉറവിടങ്ങൾ ചേർക്കുന്നത്", "Joining the two sources")) }
                p { (lang.pick(
                    "കിഫ്ബിയുടെ ഈ രണ്ട് താളുകൾക്കും പൊതുവായ തിരിച്ചറിയൽ നമ്പറില്ല. സ്റ്റാറ്റസ് താളിലെ “പദ്ധതി” ഡാഷ്ബോർഡിലെ ഒരു ഉപപദ്ധതിയാണ്; അതിലെ “പ്രവൃത്തികൾ” ഡാഷ്ബോർഡിലെ കരാർ പാക്കേജുകളും. അനുവദിച്ച തുക (രൂപ വരെ കൃത്യമായി), വകുപ്പ്, നിർവഹണ സ്ഥാപനം എന്നിവ മൂന്നും ഒന്നാണെങ്കിൽ, അങ്ങനെ യോജിക്കുന്നത് ഒന്നു മാത്രമാണെങ്കിൽ, ഞങ്ങൾ അവ ചേർക്കുന്നു. സ്ഥാപനത്തിന്റെ പേര് വ്യത്യസ്തമാണെങ്കിൽ പേരുകളിലെ വാക്കുകൾ ഭൂരിഭാഗവും യോജിക്കണം. ഒന്നിലധികം സാധ്യതകളുണ്ടെങ്കിൽ ചേർക്കില്ല. പ്രവൃത്തികളെ പാക്കേജുകളുമായി ചേർക്കുന്നത് പ്രസിദ്ധീകരിച്ച പേര് അതേപടി ഒന്നാണെങ്കിൽ മാത്രം.",
                    "KIIFB's two pages share no identifier. A “project” on the status page is a sub-project on the dashboard, and its “works” are the dashboard's contract packages. We join them when the approved amount (to the rupee), the department and the implementing agency all agree and only one record fits. Where the agency is named differently, most of the words in the names must agree. Where more than one record could fit, we do not join. A work is joined to a package only when the published titles are identical.",
                )) }
                p { (lang.pick(
                    "ഒരു പ്രവൃത്തിക്ക് അനുവദിച്ചതിനേക്കാൾ 1 ശതമാനത്തിലധികം കൂടുതൽ നൽകിയതായി കിഫ്ബി കാണിക്കുന്നിടത്ത് ഞങ്ങൾ അത് എടുത്തുപറയുന്നു. അതിലും ചെറിയ വ്യത്യാസങ്ങൾ പൈസയുടെ കണക്കാണ്. ഇത് തെറ്റ് നടന്നതിന്റെ തെളിവല്ല; പുതുക്കിയ അനുമതി താളിൽ വരാത്തതാകാം.",
                    "Where KIIFB shows a work paid more than 1% above what was approved for it, we say so. Smaller differences are paise and rounding. This is not evidence of wrongdoing; a revised approval may simply not be shown on the page.",
                )) }

                h2 { (lang.pick("പരിമിതികൾ", "Limitations")) }
                @match lang {
                    Lang::Ml => ul {
                        li { "തുകകൾ ഡാഷ്ബോർഡിൽ ഉള്ളതുപോലെയാണ്. പൂജ്യം എന്ന് കാണിച്ചവ “പ്രസിദ്ധീകരിച്ചിട്ടില്ല” എന്ന് ഞങ്ങൾ കണക്കാക്കുന്നു." }
                        li { "കണക്കാക്കിയ തുക കിഫ്ബി നൽകുന്നത് ഉപപദ്ധതിയുടെ തലത്തിലാണ്. ഒരു ഉപപദ്ധതിക്ക് കീഴിൽ പല കരാർ പാക്കേജുകളുണ്ടെങ്കിൽ എല്ലാറ്റിലും അതേ തുക ആവർത്തിക്കും; അങ്ങനെയുള്ളിടത്ത് ഞങ്ങൾ അത് വ്യക്തമാക്കുന്നു, തുകകൾ കൂട്ടുന്നില്ല." }
                        li { "കരാറുകാരൻ, തീയതികൾ, പുരോഗതി എന്നിവ റോഡ്, പാലം പ്രവൃത്തികൾക്ക് മാത്രമേ ഡാഷ്ബോർഡിൽ ഉള്ളൂ." }
                        li { "ഭൂപടത്തിലെ സ്ഥാനങ്ങൾ കിഫ്ബി നൽകിയവയാണ്; ചിലത് തെറ്റായിരിക്കാം." }
                        li { "സ്റ്റാറ്റസ് താളിലെ ചില പദ്ധതികൾ പല ജില്ലകളിലായുള്ളവയാണ്; അവയുടെ തുകകൾ എറണാകുളത്തിന് മാത്രമുള്ളതല്ല. ഡാഷ്ബോർഡിലെ “ചെലവും” സ്റ്റാറ്റസ് താളിലെ “നൽകിയ തുകയും” വ്യത്യസ്ത സംഖ്യകളാണ്; രണ്ടും അതത് ഉറവിടത്തിൽ ഉള്ളതുപോലെ കാണിക്കുന്നു." }
                        li { "“ചെലവ് വർധന”, “പുതിയ വിവരമില്ല” എന്നീ സൂചനകൾ ഞങ്ങൾ ശേഖരണം തുടങ്ങിയ ശേഷമുള്ള മാറ്റങ്ങളെ മാത്രം അടിസ്ഥാനമാക്കിയാണ്." }
                        li { "പദ്ധതികളുടെ പേരുകൾ കിഫ്ബി ഇംഗ്ലീഷിലാണ് പ്രസിദ്ധീകരിക്കുന്നത്. തലക്കെട്ടിൽ ഞങ്ങൾ ഫയൽ കോഡുകൾ നീക്കുകയും വലിയക്ഷരങ്ങൾ സാധാരണ രൂപത്തിലാക്കുകയും ചെയ്യുന്നു; യഥാർത്ഥ പേര് താളിൽ കാണാം." }
                        li { "മണ്ഡലങ്ങളുടെ പേരുകൾ കിഫ്ബി പല രീതിയിൽ എഴുതുന്നു (Vypeen, Vypin, Vyppin). ഞങ്ങൾ അവ ഒന്നാക്കുന്നു. എം.എൽ.എമാരുടെ പേരുകൾ കിഫ്ബി പട്ടികയിൽ ഉള്ളതുപോലെയാണ്." }
                    },
                    Lang::En => ul {
                        li { "Amounts are as the dashboard states them. Where it shows zero, we treat the figure as not published." }
                        li { "KIIFB states the estimated amount per sub-project. Where several contract packages sit under one sub-project, each repeats the same estimate; we say so on the page and never add those estimates up." }
                        li { "The dashboard gives contractor, dates and progress only for road and bridge works." }
                        li { "Map locations are the ones KIIFB records; some may be wrong." }
                        li { "Some projects on the status page span several districts, so their amounts are not Ernakulam's alone. The dashboard's “expenditure” and the status page's “released” are different figures; we show each as its source states it." }
                        li { "“Cost escalation” and “No recent update” rely only on changes since we began collecting." }
                        li { "KIIFB publishes project names in English. In headlines we drop filing codes and calm all-capital titles; the name as published is shown on the page." }
                        li { "KIIFB spells constituencies several ways (Vypeen, Vypin, Vyppin). We merge them. MLA names are as KIIFB lists them." }
                    },
                }

                h2 { (lang.pick("തിരുത്തലുകൾ", "Corrections")) }
                p { (lang.pick(
                    "ഞങ്ങൾ കാണിക്കുന്നത് ഉറവിടത്തിൽ നിന്ന് വ്യത്യസ്തമാണെങ്കിൽ അറിയിക്കുക. ഉറവിടവുമായി ഒത്തുനോക്കി ഞങ്ങൾ തിരുത്തും.",
                    "If what we show differs from the source, tell us. We will check it against the source and correct it.",
                )) }
            }
        }
    };
    layout(&page(lang, t.methodology_title, lead, "/methodology", origin, Nav::Methodology, last_checked), body)
}

pub fn data(lang: Lang, origin: &str, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let lead = lang.pick(
        "ഈ സൈറ്റിലെ എല്ലാ വിവരങ്ങളും യന്ത്രങ്ങൾക്ക് വായിക്കാവുന്ന രൂപത്തിൽ ലഭ്യമാണ്. ലോഗിൻ വേണ്ട.",
        "Everything on this site is available in machine-readable form. No login is needed.",
    );
    let endpoints: [(&str, &str, &str, &str); 5] = [
        ("/api/v1/projects.csv", "CSV", "സ്പ്രെഡ്ഷീറ്റിൽ തുറക്കാൻ: എല്ലാ പദ്ധതികളും", "All projects, for spreadsheets"),
        ("/api/v1/projects", "JSON", "എല്ലാ പദ്ധതികളും, പ്രവൃത്തികൾ ഉൾപ്പെടെ", "All projects, with their works"),
        ("/api/v1/projects.geojson", "GeoJSON", "പദ്ധതി സ്ഥാനങ്ങൾ", "Project locations"),
        ("/api/v1/funding.csv", "CSV", "അനുവദിച്ചതും നൽകിയതും: ഓരോ പ്രവൃത്തിയും ഒരു വരി", "Approved and paid, one row per work"),
        ("/api/v1/funding", "JSON", "അനുവദിച്ചതും നൽകിയതും, പ്രവൃത്തികൾ ഉൾപ്പെടെ", "Approved and released, with works"),
    ];
    let body = html! {
        (head(lang, t.data_title, lead))
        div.wrap {
            section.sec {
                div.tiles {
                    @for (path, kind, ml, en) in endpoints {
                        a.tile href=(path) {
                            (icon(icons::DOWNLOAD))
                            b { (kind) }
                            span.small.muted { (lang.pick(ml, en)) }
                            span.code { (path) }
                        }
                    }
                }
            }
            div.prose {
                h2 { (lang.pick("ഒരു പദ്ധതി മാത്രം", "One project")) }
                p {
                    span.code { "/api/v1/projects/{code}" } " "
                    (lang.pick(
                        "ഒരു പദ്ധതിയുടെ രേഖ, സൂചനകൾ, മാറ്റങ്ങൾ, ഉറവിടം എന്നിവ നൽകുന്നു.",
                        "returns one project's record, flags, recorded changes and source.",
                    ))
                }
                h2 { (lang.pick("ഉപയോഗിക്കുമ്പോൾ", "Using the data")) }
                ul {
                    li { (lang.pick("തുകകൾ മുഴുവൻ രൂപയിലാണ്. തീയതികൾ yyyy-mm-dd രൂപത്തിൽ.", "Amounts are whole rupees. Dates are yyyy-mm-dd.")) }
                    li { (lang.pick(
                        "“estimated_amount” ഉപപദ്ധതിയുടേതാണ്; “estimate_shared_by” ഒന്നിൽ കൂടുതലാണെങ്കിൽ അത് കൂട്ടരുത്.",
                        "“estimated_amount” belongs to the sub-project; do not sum it where “estimate_shared_by” is greater than one.",
                    )) }
                    li { (lang.pick(
                        "“funding” ഫയലുകൾ കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളിൽ നിന്നാണ്. “map_group” ഉണ്ടെങ്കിൽ അത് ഞങ്ങൾ ചേർത്ത ഡാഷ്ബോർഡ് ഉപപദ്ധതിയാണ്; ചേർത്തത് ഞങ്ങളാണ്, കിഫ്ബിയല്ല.",
                        "The “funding” files come from KIIFB's project status page. Where “map_group” is set, it is the dashboard sub-project we joined it to; the join is ours, not KIIFB's.",
                    )) }
                    li { (lang.pick(
                        "ഉറവിടമായി കിഫ്ബിയെയും സമാഹരിച്ചത് കണക്ക് എന്നും പരാമർശിക്കുക.",
                        "Credit KIIFB as the source and Kanakku as the compiler.",
                    )) }
                }
            }
        }
    };
    layout(&page(lang, t.data_title, lead, "/data", origin, Nav::Data, last_checked), body)
}

pub fn map(lang: Lang, origin: &str, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let head_markup = html! {
        link rel="stylesheet" href="/vendor/leaflet.css";
        script defer src="/vendor/leaflet.js" {}
        script defer src="/vendor/protomaps-leaflet.js" {}
        script defer src="/map.js" {}
    };
    let body = html! {
        (head(lang, t.map_title, t.map_intro))
        div.wrap {
            div #map data-lang=(lang.code()) data-prefix=(lang.prefix()) { p.small { (t.map_loading) } }
            p.legend {
                span { i.dot.f {} (t.map_legend_flagged) }
                span { i.dot {} (t.map_legend_clear) }
                a.more href=(format!("{}/projects", lang.prefix())) { (lang.pick("പട്ടികയായി കാണുക", "See as a list")) (icon(icons::ARROW_RIGHT)) }
            }
        }
    };
    let mut p = page(lang, t.map_title, t.map_intro, "/map", origin, Nav::Map, last_checked);
    p.head = head_markup;
    layout(&p, body)
}

pub fn not_found(lang: Lang, origin: &str) -> String {
    let t = lang.t();
    let body = html! {
        (head(lang, t.not_found_title, t.not_found_body))
        div.wrap {
            p.actions.sec {
                a.btn href=(format!("{}/projects", lang.prefix())) { (t.back_to_list) }
                a.btn.ghost href=(format!("{}/", lang.prefix())) { (t.site_name) }
            }
        }
    };
    layout(&page(lang, t.not_found_title, t.not_found_body, "/", origin, Nav::None, None), body)
}

fn page<'a>(
    lang: Lang,
    title: &'a str,
    description: &'a str,
    path: &'a str,
    origin: &'a str,
    nav: Nav,
    last_checked: Option<&'a str>,
) -> Page<'a> {
    Page { lang, title, description, path, origin, nav, last_checked, head: Markup::default() }
}
