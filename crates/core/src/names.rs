//! Canonical names. KIIFB spells the same constituency several ways; readers should see one.

use crate::constituencies::CONSTITUENCIES;
use crate::i18n::Lang;

/// Other spellings KIIFB uses for a constituency (lower case), and the one we show.
const ALIASES: &[(&str, &str)] = &[
    ("angamali", "Angamaly"),
    ("kalamasserry", "Kalamassery"),
    ("kalamasery", "Kalamassery"),
    ("cochin", "Kochi"),
    ("kunnathunadu", "Kunnathunad"),
    ("moovattupuzha", "Muvattupuzha"),
    ("paravoor", "Paravur"),
    ("north paravur", "Paravur"),
    ("north paravoor", "Paravur"),
    ("perumbavur", "Perumbavoor"),
    ("piravam", "Piravom"),
    ("thrikkakkara", "Thrikkakara"),
    ("thrippunithura", "Thripunithura"),
    ("tripunithura", "Thripunithura"),
    ("thrippunithara", "Thripunithura"),
    ("vypeen", "Vypin"),
    ("vyppin", "Vypin"),
];

/// One spelling per constituency: drops reservation suffixes such as "(SC)", repairs
/// mis-decoded non-breaking spaces, and maps known alternative spellings.
pub fn canonical_constituency(raw: &str) -> String {
    let mut name = raw.replace(['\u{a0}', '\u{c2}'], " ");
    if let Some(open) = name.find('(') {
        name.truncate(open);
    }
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let key = name.to_lowercase();
    if let Some((_, canonical)) = ALIASES.iter().find(|(alias, _)| *alias == key) {
        return (*canonical).to_string();
    }
    match CONSTITUENCIES.iter().find(|(known, _, _)| known.eq_ignore_ascii_case(&name)) {
        Some((known, _, _)) => (*known).to_string(),
        None => name,
    }
}

pub fn constituency_ml(canonical: &str) -> Option<&'static str> {
    CONSTITUENCIES.iter().find(|(name, _, _)| *name == canonical).map(|(_, ml, _)| *ml)
}

/// The district a constituency lies in.
pub fn constituency_district(canonical: &str) -> Option<&'static str> {
    CONSTITUENCIES.iter().find(|(name, _, _)| *name == canonical).map(|(_, _, district)| *district)
}

/// The constituency name to show in a language, falling back to English.
pub fn constituency_label(lang: Lang, canonical: &str) -> &str {
    match lang {
        Lang::Ml => constituency_ml(canonical).unwrap_or(canonical),
        Lang::En => canonical,
    }
}

/// Stands for "every district" wherever one district could be named.
pub const ALL_DISTRICTS: &str = "*";

/// Kerala's districts, north to south as KIIFB numbers them is not needed here; this is south to north.
/// Each has its Malayalam name and the other spellings the sources use (lower case).
const DISTRICTS: &[(&str, &str, &[&str])] = &[
    ("Thiruvananthapuram", "തിരുവനന്തപുരം", &["trivandrum", "tvm", "tvpm", "thiruvananthapuruam"]),
    ("Kollam", "കൊല്ലം", &["quilon"]),
    ("Pathanamthitta", "പത്തനംതിട്ട", &[]),
    ("Alappuzha", "ആലപ്പുഴ", &["alleppey"]),
    ("Kottayam", "കോട്ടയം", &[]),
    ("Idukki", "ഇടുക്കി", &[]),
    ("Ernakulam", "എറണാകുളം", &[]),
    ("Thrissur", "തൃശ്ശൂർ", &["trichur"]),
    ("Palakkad", "പാലക്കാട്", &["palghat"]),
    ("Malappuram", "മലപ്പുറം", &[]),
    ("Kozhikode", "കോഴിക്കോട്", &["kozhikkode", "calicut"]),
    ("Wayanad", "വയനാട്", &["wayanadu"]),
    ("Kannur", "കണ്ണൂർ", &["cannanore"]),
    ("Kasaragod", "കാസർഗോഡ്", &["kasargod", "kasragod", "kasaragode"]),
];

pub fn districts() -> impl Iterator<Item = &'static str> {
    DISTRICTS.iter().map(|(name, _, _)| *name)
}

/// The district a name or spelling refers to.
pub fn canonical_district(raw: &str) -> Option<&'static str> {
    let key = raw.trim().to_lowercase();
    DISTRICTS.iter().find(|(name, _, aliases)| name.eq_ignore_ascii_case(&key) || aliases.contains(&key.as_str())).map(|(name, _, _)| *name)
}

/// The district name to show in a language. An empty name means the source stated none.
pub fn district_label(lang: Lang, name: &str) -> &str {
    if name.is_empty() {
        return lang.pick("ജില്ല രേഖപ്പെടുത്താത്തവ", "District not stated");
    }
    match lang {
        Lang::Ml => DISTRICTS.iter().find(|(known, _, _)| *known == name).map(|(_, ml, _)| *ml).unwrap_or(name),
        Lang::En => name,
    }
}

/// The district a URL segment such as `ernakulam` names.
pub fn district_from_slug(slug: &str) -> Option<&'static str> {
    DISTRICTS.iter().find(|(name, _, _)| name.eq_ignore_ascii_case(slug)).map(|(name, _, _)| *name)
}

const DEPARTMENTS: &[(&str, &str)] = &[
    ("Higher Education Department", "ഉന്നത വിദ്യാഭ്യാസ വകുപ്പ്"),
    ("General Education Department", "പൊതു വിദ്യാഭ്യാസ വകുപ്പ്"),
    ("Public Works Department", "പൊതുമരാമത്ത് വകുപ്പ്"),
    ("Health & Family Welfare", "ആരോഗ്യ കുടുംബക്ഷേമ വകുപ്പ്"),
    ("Water Resources Department", "ജലവിഭവ വകുപ്പ്"),
    ("Information Technology Department", "വിവരസാങ്കേതിക വകുപ്പ്"),
    ("Coastal Shipping & Inland Navigation Department", "തീരദേശ കപ്പൽ, ഉൾനാടൻ ജലഗതാഗത വകുപ്പ്"),
    ("Fisheries Department", "ഫിഷറീസ് വകുപ്പ്"),
    ("SC&ST Department", "പട്ടികജാതി പട്ടികവർഗ വികസന വകുപ്പ്"),
    ("Local Self Government Department", "തദ്ദേശ സ്വയംഭരണ വകുപ്പ്"),
    ("Registration Department", "രജിസ്ട്രേഷൻ വകുപ്പ്"),
    ("Power Department", "ഊർജ്ജ വകുപ്പ്"),
    ("Sports & Youth Affairs Department", "കായിക യുവജനകാര്യ വകുപ്പ്"),
    ("Agriculture Department", "കൃഷി വകുപ്പ്"),
    ("Tourism Department", "ടൂറിസം വകുപ്പ്"),
    ("Industries Department", "വ്യവസായ വകുപ്പ്"),
    ("Forest Department", "വനം വകുപ്പ്"),
    ("Transport Department", "ഗതാഗത വകുപ്പ്"),
    ("Revenue Department", "റവന്യൂ വകുപ്പ്"),
    ("Home Department", "ആഭ്യന്തര വകുപ്പ്"),
    ("Cultural Affairs Department", "സാംസ്കാരിക വകുപ്പ്"),
];

/// The department name to show in a language. English names lose the redundant "Department".
pub fn department_label(lang: Lang, name: &str) -> String {
    match lang {
        Lang::Ml => DEPARTMENTS
            .iter()
            .find(|(en, _)| en.eq_ignore_ascii_case(name))
            .map(|(_, ml)| (*ml).to_string())
            .unwrap_or_else(|| name.to_string()),
        Lang::En => name.strip_suffix(" Department").unwrap_or(name).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kiifb_spellings_collapse_to_one_name() {
        for raw in ["Vypeen", "Vypin", "Vyppin", " vypin "] {
            assert_eq!(canonical_constituency(raw), "Vypin", "{raw}");
        }
        for raw in ["Kunnathunad", "Kunnathunad (SC)", "Kunnathunad\u{a0}(SC)", "Kunnathunad\u{c2}\u{a0}(SC)"] {
            assert_eq!(canonical_constituency(raw), "Kunnathunad", "{raw:?}");
        }
        assert_eq!(canonical_constituency("Thrippunithura"), "Thripunithura");
        assert_eq!(canonical_constituency("Kalamassery"), "Kalamassery");
    }

    #[test]
    fn unknown_constituencies_pass_through_cleaned() {
        assert_eq!(canonical_constituency("  Ollur  (66) "), "Ollur");
        assert_eq!(canonical_constituency("  Mahe  (1) "), "Mahe");
        assert_eq!(constituency_ml("Mahe"), None);
        assert_eq!(constituency_label(Lang::Ml, "Mahe"), "Mahe");
    }

    #[test]
    fn labels_follow_the_language() {
        assert_eq!(constituency_label(Lang::Ml, "Aluva"), "ആലുവ");
        assert_eq!(constituency_label(Lang::En, "Aluva"), "Aluva");
        assert_eq!(department_label(Lang::En, "Public Works Department"), "Public Works");
        assert_eq!(department_label(Lang::Ml, "Public Works Department"), "പൊതുമരാമത്ത് വകുപ്പ്");
        assert_eq!(department_label(Lang::En, "Health & Family Welfare"), "Health & Family Welfare");
        assert_eq!(department_label(Lang::Ml, "Department of Magic"), "Department of Magic");
    }

    #[test]
    fn every_alias_is_lower_case_and_points_at_a_known_constituency() {
        for (alias, canonical) in ALIASES {
            assert_eq!(*alias, alias.to_lowercase());
            assert!(constituency_ml(canonical).is_some(), "{canonical}");
            assert_eq!(canonical_constituency(alias), *canonical);
        }
        for (name, _, district) in CONSTITUENCIES {
            assert_eq!(canonical_constituency(name), *name);
            assert!(canonical_district(district).is_some(), "{name} is in {district}");
        }
        assert_eq!(CONSTITUENCIES.len(), 140);
    }

    #[test]
    fn districts_resolve_from_the_spellings_the_sources_use() {
        assert_eq!(canonical_district(" kasargod "), Some("Kasaragod"));
        assert_eq!(canonical_district("KOZHIKKODE"), Some("Kozhikode"));
        assert_eq!(canonical_district("TVM"), Some("Thiruvananthapuram"));
        assert_eq!(canonical_district("Mahe"), None);
        assert_eq!(district_from_slug("ernakulam"), Some("Ernakulam"));
        assert_eq!(district_label(Lang::Ml, "Wayanad"), "വയനാട്");
        assert_eq!(district_label(Lang::En, ""), "District not stated");
        assert_eq!(districts().count(), 14);
        assert_eq!(constituency_district("Ollur"), Some("Thrissur"));
        assert_eq!(constituency_label(Lang::Ml, "Ollur"), "ഒല്ലൂർ");
    }
}
