//! What the visit counter keeps about where readers come from: a site's name, a campaign tag on
//! a link we handed out, and which individual page was read. Never a full address, never anything
//! about the reader.

/// Sites grouped under one name, so "where readers come from" reads as a short list.
/// Matched against the referring host with any `www.`, `m.` or `l.` prefix removed.
const SOURCES: [(&str, &str); 22] = [
    ("groups.google.com", "Google Groups"),
    ("news.google.com", "Google News"),
    ("google.", "Google search"),
    ("bing.com", "Bing"),
    ("duckduckgo.com", "DuckDuckGo"),
    ("whatsapp.com", "WhatsApp"),
    ("wa.me", "WhatsApp"),
    ("facebook.com", "Facebook"),
    ("fb.com", "Facebook"),
    ("instagram.com", "Instagram"),
    ("t.co", "X"),
    ("x.com", "X"),
    ("twitter.com", "X"),
    ("reddit.com", "Reddit"),
    ("t.me", "Telegram"),
    ("telegram.org", "Telegram"),
    ("linkedin.com", "LinkedIn"),
    ("lnkd.in", "LinkedIn"),
    ("youtube.com", "YouTube"),
    ("github.com", "GitHub"),
    ("news.ycombinator.com", "Hacker News"),
    ("matrix.to", "Matrix"),
];

/// The source to count for a `Referer` header, or `None` for no referrer or a page on our own
/// site (`own` lists our host names).
pub fn referrer_source(referrer: &str, own: &[&str]) -> Option<String> {
    let (scheme, rest) = referrer.trim().split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("http") {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next()?.rsplit('@').next()?.split(':').next()?.to_ascii_lowercase();
    if host.is_empty() || own.iter().any(|o| host == *o || host.ends_with(&format!(".{o}"))) || host.ends_with(".workers.dev") {
        return None;
    }
    let bare = ["www.", "m.", "l.", "lm.", "mobile.", "web.", "out."].iter().fold(host.as_str(), |h, p| h.strip_prefix(p).unwrap_or(h));
    for (pattern, name) in SOURCES {
        let hit = if pattern.ends_with('.') {
            bare.starts_with(pattern) || bare.contains(&format!(".{pattern}"))
        } else {
            bare == pattern || bare.ends_with(&format!(".{pattern}"))
        };
        if hit {
            return Some(name.to_string());
        }
    }
    let name: String = bare.chars().take(60).collect();
    (name.contains('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))).then_some(name)
}

/// A campaign tag from `?ref=` or `?utm_source=`: lower case letters, digits, `-` and `_`, at most
/// 32 characters. Anything else is not a tag we handed out.
pub fn campaign_tag(raw: &str) -> Option<String> {
    let tag = raw.trim().to_ascii_lowercase();
    (!tag.is_empty() && tag.len() <= 32 && tag.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))).then_some(tag)
}

/// The individual page to count, for paths (without the language prefix) of one project,
/// payment record, district, contractor or agency. Lists and standing pages are counted by kind.
pub fn page_key(path: &str) -> Option<String> {
    let rest = ["/p/", "/f/", "/d/", "/c/", "/a/"].iter().find_map(|prefix| path.strip_prefix(prefix).map(|r| (prefix, r)))?;
    let (prefix, id) = rest;
    (!id.is_empty() && id.len() <= 90 && !id.contains('/')).then(|| format!("{prefix}{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: [&str; 1] = ["keralakanakku.com"];

    #[test]
    fn referrers_become_short_names() {
        assert_eq!(referrer_source("https://www.google.com/", &OWN).as_deref(), Some("Google search"));
        assert_eq!(referrer_source("https://www.google.co.in/search?q=kiifb", &OWN).as_deref(), Some("Google search"));
        assert_eq!(referrer_source("https://groups.google.com/g/datameet/c/abc", &OWN).as_deref(), Some("Google Groups"));
        assert_eq!(referrer_source("https://l.facebook.com/l.php?u=x", &OWN).as_deref(), Some("Facebook"));
        assert_eq!(referrer_source("https://lm.facebook.com/", &OWN).as_deref(), Some("Facebook"));
        assert_eq!(referrer_source("https://t.co/abc", &OWN).as_deref(), Some("X"));
        assert_eq!(referrer_source("https://web.whatsapp.com/", &OWN).as_deref(), Some("WhatsApp"));
        assert_eq!(referrer_source("https://out.reddit.com/t3_x", &OWN).as_deref(), Some("Reddit"));
        assert_eq!(referrer_source("https://www.thenewsminute.com/kerala/story", &OWN).as_deref(), Some("thenewsminute.com"));
    }

    #[test]
    fn own_pages_and_junk_are_not_sources() {
        assert_eq!(referrer_source("", &OWN), None);
        assert_eq!(referrer_source("https://keralakanakku.com/en/projects", &OWN), None);
        assert_eq!(referrer_source("https://www.keralakanakku.com/", &OWN), None);
        assert_eq!(referrer_source("https://kanakku.thesolohacker47.workers.dev/", &OWN), None);
        assert_eq!(referrer_source("not a url", &OWN), None);
        assert_eq!(referrer_source("android-app://com.google.android.gm/", &OWN), None, "an app id, not a site");
        assert_eq!(referrer_source("https://user:pass@example.org:8443/x", &OWN).as_deref(), Some("example.org"));
    }

    #[test]
    fn campaign_tags_are_tidy() {
        assert_eq!(campaign_tag("DataMeet").as_deref(), Some("datameet"));
        assert_eq!(campaign_tag("osm-kerala_2026").as_deref(), Some("osm-kerala_2026"));
        assert_eq!(campaign_tag(""), None);
        assert_eq!(campaign_tag("<script>"), None);
        assert_eq!(campaign_tag(&"x".repeat(33)), None);
    }

    #[test]
    fn only_single_record_pages_are_counted_individually() {
        assert_eq!(page_key("/p/PWD015-69-03").as_deref(), Some("/p/PWD015-69-03"));
        assert_eq!(page_key("/d/ernakulam").as_deref(), Some("/d/ernakulam"));
        assert_eq!(page_key("/projects"), None);
        assert_eq!(page_key("/p/"), None);
        assert_eq!(page_key("/p/a/b"), None);
    }
}
