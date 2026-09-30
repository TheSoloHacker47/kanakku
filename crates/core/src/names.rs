//! Canonical names. KIIFB spells the same constituency several ways; readers should see one.

use crate::i18n::Lang;

/// Ernakulam's assembly constituencies: canonical English name, Malayalam name, and the
/// other spellings KIIFB uses for it (lower case).
const CONSTITUENCIES: &[(&str, &str, &[&str])] = &[
    ("Aluva", "ആലുവ", &[]),
    ("Angamaly", "അങ്കമാലി", &["angamali"]),
    ("Ernakulam", "എറണാകുളം", &[]),
    ("Kalamassery", "കളമശ്ശേരി", &["kalamasserry", "kalamasery"]),
    ("Kochi", "കൊച്ചി", &["cochin"]),
    ("Kothamangalam", "കോതമംഗലം", &[]),
    ("Kunnathunad", "കുന്നത്തുനാട്", &["kunnathunadu"]),
    ("Muvattupuzha", "മൂവാറ്റുപുഴ", &["moovattupuzha"]),
    ("Paravur", "പറവൂർ", &["paravoor", "north paravur", "north paravoor"]),
    ("Perumbavoor", "പെരുമ്പാവൂർ", &["perumbavur"]),
    ("Piravom", "പിറവം", &["piravam"]),
    ("Thrikkakara", "തൃക്കാക്കര", &["thrikkakkara"]),
    ("Thripunithura", "തൃപ്പൂണിത്തുറ", &["thrippunithura", "tripunithura", "thrippunithara"]),
    ("Vypin", "വൈപ്പിൻ", &["vypeen", "vyppin", "vypeen"]),
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
    for (canonical, _, aliases) in CONSTITUENCIES {
        if canonical.eq_ignore_ascii_case(&name) || aliases.contains(&key.as_str()) {
            return (*canonical).to_string();
        }
    }
    name
}

pub fn constituency_ml(canonical: &str) -> Option<&'static str> {
    CONSTITUENCIES.iter().find(|(name, _, _)| *name == canonical).map(|(_, ml, _)| *ml)
}

/// The constituency name to show in a language, falling back to English.
pub fn constituency_label(lang: Lang, canonical: &str) -> &str {
    match lang {
        Lang::Ml => constituency_ml(canonical).unwrap_or(canonical),
        Lang::En => canonical,
    }
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
        assert_eq!(constituency_ml("Ollur"), None);
        assert_eq!(constituency_label(Lang::Ml, "Ollur"), "Ollur");
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
    fn every_alias_is_lower_case_and_every_name_is_its_own_canonical_form() {
        for (name, _, aliases) in CONSTITUENCIES {
            assert_eq!(canonical_constituency(name), *name);
            for alias in *aliases {
                assert_eq!(*alias, alias.to_lowercase());
                assert_eq!(canonical_constituency(alias), *name);
            }
        }
    }
}
