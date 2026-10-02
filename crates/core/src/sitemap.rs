//! `/sitemap.xml`: every page a crawler should know about, in both languages.
//!
//! Crawlers are kept off query-string URLs (robots.txt), where filter combinations multiply
//! without end. The sitemap is how they still reach every project, payment and contractor page.

use crate::names;

/// Pages that exist once per language.
const FIXED: [&str; 11] =
    ["/", "/projects", "/funding", "/liability", "/contractors", "/agencies", "/map", "/methodology", "/data", "/about", "/press"];

/// The site-relative paths, without the language prefix.
pub fn paths(projects: &[String], funded: &[String], contractors: &[String], agencies: &[String]) -> Vec<String> {
    let mut out: Vec<String> = FIXED.iter().map(|p| p.to_string()).collect();
    out.extend(names::districts().map(|d| format!("/d/{}", d.to_ascii_lowercase())));
    out.extend(projects.iter().map(|k| format!("/p/{k}")));
    out.extend(funded.iter().map(|k| format!("/f/{k}")));
    out.extend(contractors.iter().map(|k| format!("/c/{k}")));
    out.extend(agencies.iter().map(|k| format!("/a/{k}")));
    out
}

/// The XML for `origin` (such as `https://keralakanakku.com`). Malayalam lives at the root, English under `/en`.
pub fn xml(origin: &str, paths: &[String]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for path in paths {
        let en = if path == "/" { "/en".to_string() } else { format!("/en{path}") };
        for p in [path.as_str(), en.as_str()] {
            out.push_str("<url><loc>");
            out.push_str(&escape(&format!("{origin}{p}")));
            out.push_str("</loc></url>\n");
        }
    }
    out.push_str("</urlset>\n");
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_appears_in_both_languages() {
        let paths = paths(&["PWD015-15".into()], &["f4ud6wd6wa9z".into()], &["tabdulrahiman".into()], &["krfb".into()]);
        assert!(paths.contains(&"/d/ernakulam".to_string()));
        let xml = xml("https://keralakanakku.com", &paths);
        for loc in [
            "https://keralakanakku.com/",
            "https://keralakanakku.com/en",
            "https://keralakanakku.com/p/PWD015-15",
            "https://keralakanakku.com/en/p/PWD015-15",
            "https://keralakanakku.com/en/f/f4ud6wd6wa9z",
            "https://keralakanakku.com/c/tabdulrahiman",
            "https://keralakanakku.com/en/a/krfb",
            "https://keralakanakku.com/en/projects",
        ] {
            assert!(xml.contains(&format!("<loc>{loc}</loc>")), "{loc}");
        }
        assert_eq!(xml.matches("<url>").count(), paths.len() * 2);
    }

    #[test]
    fn locations_are_escaped() {
        assert!(xml("https://x", &["/c/a&b".into()]).contains("<loc>https://x/c/a&amp;b</loc>"));
    }
}
