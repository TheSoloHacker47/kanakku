//! Server-rendered pages. Templates are maud macros, compiled to plain string writes.

use kanakku_core::i18n::Lang;
use kanakku_core::Date;
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::http::CSS;

pub mod list;
pub mod pages;
pub mod project;

#[derive(Clone, Copy, PartialEq)]
pub enum Nav {
    Projects,
    Map,
    Methodology,
    Data,
    None,
}

pub struct Page<'a> {
    pub lang: Lang,
    pub title: &'a str,
    pub description: &'a str,
    /// Path and query of this page without the language prefix, e.g. `/p/PWD016-05-01`.
    pub path: &'a str,
    pub nav: Nav,
    /// When the source was last checked, as a UTC timestamp.
    pub last_checked: Option<&'a str>,
    /// Extra `<head>` markup, used by the map page for its script and stylesheet.
    pub head: Markup,
}

pub fn layout(page: &Page, body: Markup) -> String {
    let lang = page.lang;
    let t = lang.t();
    let other = lang.other();
    let href = |path: &str| format!("{}{path}", lang.prefix());
    let link = |nav: Nav, path: &str, label: &str| {
        html! { a href=(href(path)) aria-current=[(page.nav == nav).then_some("page")] { (label) } }
    };
    let checked = page.last_checked.and_then(Date::from_utc_timestamp_ist).map(Date::to_dmy);

    html! {
        (DOCTYPE)
        html lang=(lang.code()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (page.title) " · " (t.site_name) }
                meta name="description" content=(page.description);
                link rel="icon" href="/favicon.svg" type="image/svg+xml";
                link rel="alternate" hreflang=(other.code()) href=(format!("{}{}", other.prefix(), page.path));
                style { (PreEscaped(CSS)) }
                (page.head)
            }
            body {
                a.skip href="#main" { (t.skip_to_content) }
                header.top {
                    div.wrap {
                        a.brand href=(href("/")) { (t.site_name) }
                        nav {
                            (link(Nav::Projects, "/", t.nav_projects))
                            (link(Nav::Map, "/map", t.nav_map))
                            (link(Nav::Methodology, "/methodology", t.nav_methodology))
                            (link(Nav::Data, "/data", t.nav_data))
                        }
                        a.lang href=(format!("{}{}", other.prefix(), page.path)) lang=(other.code()) hreflang=(other.code()) {
                            (other.t().lang_name)
                        }
                    }
                }
                main #main {
                    div.wrap { (body) }
                }
                footer.foot {
                    div.wrap {
                        p { (t.footer_principle) }
                        p {
                            (t.footer_source) " "
                            @match &checked {
                                Some(date) => (date),
                                None => "—",
                            }
                        }
                        p { (t.footer_corrections) }
                    }
                }
            }
        }
    }
    .into_string()
}

/// Text that stays in English on a Malayalam page (KIIFB publishes names and statuses in English).
pub fn en(text: &str) -> Markup {
    html! { span lang="en" { (text) } }
}
