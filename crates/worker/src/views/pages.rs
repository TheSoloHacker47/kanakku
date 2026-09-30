//! The static pages: methodology, open data, map shell and not-found.

use kanakku_core::flags::{FlagKind, RULES_VERSION};
use kanakku_core::i18n::{flag_label, flag_rule, Lang};
use maud::{html, Markup};

use super::{layout, Nav, Page};

pub fn methodology(lang: Lang, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let body = html! {
        h1 { (t.methodology_title) }
        div.prose {
            @match lang {
                Lang::Ml => {
                    p { "കണക്ക് ഒരു കാര്യം മാത്രമാണ് ചെയ്യുന്നത്: സർക്കാർ തന്നെ പ്രസിദ്ധീകരിക്കുന്ന പദ്ധതി വിവരങ്ങൾ ശേഖരിച്ച്, വായിക്കാവുന്ന രൂപത്തിൽ, ഉറവിടത്തോടൊപ്പം കാണിക്കുന്നു." }
                    h2 { "തത്വങ്ങൾ" }
                    ul {
                        li { "ഓരോ സംഖ്യയും അതിന്റെ ഉറവിടത്തിലേക്ക് ചൂണ്ടുന്നു. ഞങ്ങൾ ശേഖരിച്ച താളിന്റെ പകർപ്പ് സൂക്ഷിക്കുന്നു; അത് ആർക്കും ഡൗൺലോഡ് ചെയ്യാം." }
                        li { "വസ്തുതകളും സൂചനകളും മാത്രം. “14 മാസം വൈകി” എന്ന് പറയും; കാരണം ഊഹിക്കില്ല, ആരെയും കുറ്റപ്പെടുത്തില്ല." }
                        li { "എല്ലാ കക്ഷികൾക്കും എം.എൽ.എമാർക്കും തദ്ദേശ സ്ഥാപനങ്ങൾക്കും ഒരേ നിയമങ്ങൾ." }
                    }
                }
                Lang::En => {
                    p { "Kanakku does one thing: it collects the project figures the government itself publishes and shows them in a readable form, next to their source." }
                    h2 { "Principles" }
                    ul {
                        li { "Every number points to its source. We keep a copy of the page we collected it from, and anyone can download that copy." }
                        li { "Facts and flags only. We say “14 months past scheduled completion”; we do not guess at reasons or blame anyone." }
                        li { "The same rules apply to every party, MLA and local body." }
                    }
                }
            }
        }

        h2 #rules { (lang.pick("സൂചനാ നിയമങ്ങൾ", "Flag rules")) " · " (t.rule_version) " " (RULES_VERSION) }
        table {
            thead { tr { th { (t.filter_flag) } th { (lang.pick("നിയമം", "Rule")) } } }
            tbody {
                @for kind in FlagKind::ALL {
                    tr { td { (flag_label(lang, kind)) } td { (flag_rule(lang, kind)) } }
                }
            }
        }

        div.prose {
            @match lang {
                Lang::Ml => {
                    p { "ഒരു സൂചന എന്നത് പ്രസിദ്ധീകരിച്ച സംഖ്യകൾ ഒരു പരിധി കടന്നു എന്നതിന്റെ അടയാളം മാത്രമാണ്. അത് തെറ്റ് നടന്നു എന്നതിന്റെ തെളിവല്ല. ഓരോ സൂചനയിലും അതിന് ആധാരമായ സംഖ്യകളും നിയമത്തിന്റെ പതിപ്പും കാണാം." }
                    h2 { "ഡാറ്റ എവിടെ നിന്ന്" }
                    p { "കിഫ്ബിയുടെ സംയോജിത ഡാഷ്ബോർഡ് (gis.kiifb.org) ദിവസത്തിൽ ഒരിക്കൽ ഞങ്ങൾ വായിക്കുന്നു. ഉള്ളടക്കം മാറിയിട്ടുണ്ടെങ്കിൽ മാത്രം പുതിയ പകർപ്പ് സൂക്ഷിക്കും; മാറിയ ഓരോ വിവരവും പദ്ധതിയുടെ താളിൽ രേഖപ്പെടുത്തും." }
                    h2 { "പരിമിതികൾ" }
                    ul {
                        li { "തുകകൾ ഡാഷ്ബോർഡിൽ ഉള്ളതുപോലെയാണ്. പൂജ്യം എന്ന് കാണിച്ചവ “പ്രസിദ്ധീകരിച്ചിട്ടില്ല” എന്ന് ഞങ്ങൾ കണക്കാക്കുന്നു." }
                        li { "കരാറുകാരൻ, തീയതികൾ, പുരോഗതി എന്നിവ റോഡ്, പാലം പ്രവൃത്തികൾക്ക് മാത്രമേ ഡാഷ്ബോർഡിൽ ഉള്ളൂ." }
                        li { "ഭൂപടത്തിലെ സ്ഥാനങ്ങൾ കിഫ്ബി നൽകിയവയാണ്; ചിലത് തെറ്റായിരിക്കാം." }
                        li { "“ചെലവ് വർധന”, “പുതിയ വിവരമില്ല” എന്നീ സൂചനകൾ ഞങ്ങൾ ശേഖരണം തുടങ്ങിയ ശേഷമുള്ള മാറ്റങ്ങളെ മാത്രം അടിസ്ഥാനമാക്കിയാണ്." }
                        li { "പദ്ധതികളുടെ പേരുകളും നിലകളും കിഫ്ബി ഇംഗ്ലീഷിലാണ് പ്രസിദ്ധീകരിക്കുന്നത്; അവ അതേപടി കാണിക്കുന്നു." }
                    }
                    h2 { "തിരുത്തലുകൾ" }
                    p { "ഞങ്ങൾ കാണിക്കുന്നത് ഉറവിടത്തിൽ നിന്ന് വ്യത്യസ്തമാണെങ്കിൽ അറിയിക്കുക. ഉറവിടവുമായി ഒത്തുനോക്കി ഞങ്ങൾ തിരുത്തും." }
                }
                Lang::En => {
                    p { "A flag only marks that published figures crossed a threshold. It is not evidence of wrongdoing. Each flag shows the figures it rests on and the version of the rule that raised it." }
                    h2 { "Where the data comes from" }
                    p { "We read KIIFB's integrated dashboard (gis.kiifb.org) once a day. We store a new copy only when its contents have changed, and every field that changed is recorded on the project's page." }
                    h2 { "Limitations" }
                    ul {
                        li { "Amounts are as the dashboard states them. Where it shows zero, we treat the figure as not published." }
                        li { "The dashboard gives contractor, dates and progress only for road and bridge works." }
                        li { "Map locations are the ones KIIFB records; some may be wrong." }
                        li { "“Cost escalation” and “No recent update” rely only on changes since we began collecting." }
                        li { "KIIFB publishes project names and statuses in English; we show them as published." }
                    }
                    h2 { "Corrections" }
                    p { "If what we show differs from the source, tell us. We will check it against the source and correct it." }
                }
            }
        }
    };
    layout(&page(lang, t.methodology_title, "/methodology", Nav::Methodology, last_checked), body)
}

pub fn data(lang: Lang, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let endpoints: [(&str, &str, &str); 4] = [
        ("/api/v1/projects", "എല്ലാ പദ്ധതികളും, JSON", "All projects, JSON"),
        ("/api/v1/projects.csv", "എല്ലാ പദ്ധതികളും, CSV (സ്പ്രെഡ്ഷീറ്റിന്)", "All projects, CSV for spreadsheets"),
        ("/api/v1/projects.geojson", "സ്ഥാനങ്ങൾ, GeoJSON", "Locations, GeoJSON"),
        ("/api/v1/projects/{code}", "ഒരു പദ്ധതി: രേഖ, സൂചനകൾ, ഉറവിടം", "One project: record, flags and source"),
    ];
    let body = html! {
        h1 { (t.data_title) }
        p.intro {
            (lang.pick(
                "ഈ സൈറ്റിലെ എല്ലാ വിവരങ്ങളും യന്ത്രങ്ങൾക്ക് വായിക്കാവുന്ന രൂപത്തിൽ ലഭ്യമാണ്. ലോഗിൻ വേണ്ട.",
                "Everything on this site is available in machine-readable form. No login is needed.",
            ))
        }
        table {
            thead { tr { th { "URL" } th { (lang.pick("ഉള്ളടക്കം", "Contents")) } } }
            tbody {
                @for (path, ml, en) in endpoints {
                    tr {
                        td.code { @if path.contains('{') { (path) } @else { a href=(path) { (path) } } }
                        td { (lang.pick(ml, en)) }
                    }
                }
            }
        }
        div.prose {
            p {
                (lang.pick(
                    "തുകകൾ മുഴുവൻ രൂപയിലാണ്. തീയതികൾ yyyy-mm-dd രൂപത്തിൽ. ഉപയോഗിക്കുമ്പോൾ ഉറവിടമായി കിഫ്ബി ഡാഷ്ബോർഡിനെയും കണക്കിനെയും പരാമർശിക്കുക.",
                    "Amounts are whole rupees. Dates are yyyy-mm-dd. When you reuse the data, credit the KIIFB dashboard as the source and Kanakku as the compiler.",
                ))
            }
        }
    };
    layout(&page(lang, t.data_title, "/data", Nav::Data, last_checked), body)
}

pub fn map(lang: Lang, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let head = html! {
        link rel="stylesheet" href="/vendor/leaflet.css";
        script defer src="/vendor/leaflet.js" {}
        script defer src="/vendor/protomaps-leaflet.js" {}
        script defer src="/map.js" {}
    };
    let body = html! {
        h1 { (t.map_title) }
        p.intro { (t.map_intro) }
        div #map data-lang=(lang.code()) data-prefix=(lang.prefix()) { p.small { (t.map_loading) } }
        p.legend {
            span { i.dot.f {} (t.map_legend_flagged) }
            span { i.dot {} (t.map_legend_clear) }
        }
    };
    let mut p = page(lang, t.map_title, "/map", Nav::Map, last_checked);
    p.head = head;
    layout(&p, body)
}

pub fn not_found(lang: Lang) -> String {
    let t = lang.t();
    let body = html! {
        h1 { (t.not_found_title) }
        p.intro { (t.not_found_body) }
        p.figure { span { a href=(format!("{}/", lang.prefix())) { (t.back_to_list) } } }
    };
    layout(&page(lang, t.not_found_title, "/", Nav::None, None), body)
}

fn page<'a>(lang: Lang, title: &'a str, path: &'a str, nav: Nav, last_checked: Option<&'a str>) -> Page<'a> {
    Page { lang, title, description: lang.t().tagline, path, nav, last_checked, head: Markup::default() }
}
