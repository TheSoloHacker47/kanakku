//! Search in Malayalam over records that KIIFB publishes in English.
//!
//! A Malayalam word is looked up in a small dictionary of place names and common project words.
//! A word that is not there is spelled out in Latin letters, in the two or three ways KIIFB is
//! likely to have written it. Every alternative is searched for; the page shows which were used.

use crate::constituencies::CONSTITUENCIES;

/// Malayalam words and the English spellings to search for. Place names carry the variants
/// that appear in KIIFB's titles.
const WORDS: &[(&str, &[&str])] = &[
    // Districts.
    ("തിരുവനന്തപുരം", &["Thiruvananthapuram", "Trivandrum"]),
    ("കൊല്ലം", &["Kollam"]),
    ("പത്തനംതിട്ട", &["Pathanamthitta"]),
    ("ആലപ്പുഴ", &["Alappuzha"]),
    ("കോട്ടയം", &["Kottayam"]),
    ("ഇടുക്കി", &["Idukki"]),
    ("എറണാകുളം", &["Ernakulam"]),
    ("തൃശ്ശൂർ", &["Thrissur"]),
    ("തൃശൂർ", &["Thrissur"]),
    ("പാലക്കാട്", &["Palakkad"]),
    ("മലപ്പുറം", &["Malappuram"]),
    ("കോഴിക്കോട്", &["Kozhikode", "Calicut"]),
    ("വയനാട്", &["Wayanad"]),
    ("കണ്ണൂർ", &["Kannur"]),
    ("കാസർഗോഡ്", &["Kasaragod", "Kasargod"]),
    ("കാസർകോട്", &["Kasaragod", "Kasargod"]),
    // Towns and local bodies of Ernakulam that are not constituencies.
    ("കൊച്ചി", &["Kochi", "Cochin"]),
    ("കാക്കനാട്", &["Kakkanad"]),
    ("മരട്", &["Maradu"]),
    ("ഏലൂർ", &["Eloor"]),
    ("ചെല്ലാനം", &["Chellanam"]),
    ("മട്ടാഞ്ചേരി", &["Mattancherry"]),
    ("ഇടപ്പള്ളി", &["Edappally", "Edapally"]),
    ("വൈറ്റില", &["Vyttila"]),
    ("കുണ്ടന്നൂർ", &["Kundannoor", "Kundannur"]),
    ("പാലാരിവട്ടം", &["Palarivattom"]),
    ("കലൂർ", &["Kaloor"]),
    ("ചേരാനല്ലൂർ", &["Cheranallur", "Cheranelloor"]),
    ("വരാപ്പുഴ", &["Varapuzha"]),
    ("ഞാറക്കൽ", &["Njarakkal", "Njarackal"]),
    ("മുനമ്പം", &["Munambam"]),
    ("കാലടി", &["Kalady"]),
    ("നെടുമ്പാശ്ശേരി", &["Nedumbassery"]),
    ("കൂത്താട്ടുകുളം", &["Koothattukulam"]),
    ("കോലഞ്ചേരി", &["Kolenchery"]),
    ("മുളന്തുരുത്തി", &["Mulanthuruthy"]),
    ("ചോറ്റാനിക്കര", &["Chottanikkara"]),
    ("കുമ്പളങ്ങി", &["Kumbalangi", "Kumbalanghi"]),
    ("പള്ളുരുത്തി", &["Palluruthy"]),
    ("ഇടക്കൊച്ചി", &["Edakochi", "Edakkochi"]),
    ("കടമക്കുടി", &["Kadamakudy", "Kadamakkudy"]),
    ("മുളവുകാട്", &["Mulavukad"]),
    ("എളങ്കുന്നപ്പുഴ", &["Elamkunnapuzha"]),
    ("നായരമ്പലം", &["Nayarambalam"]),
    ("ചേന്ദമംഗലം", &["Chendamangalam"]),
    ("കരുമാല്ലൂർ", &["Karumalloor", "Karumallur", "Karumaloor"]),
    ("കുന്നുകര", &["Kunnukara"]),
    ("ആലങ്ങാട്", &["Alangad"]),
    ("കിഴക്കമ്പലം", &["Kizhakkambalam"]),
    ("മലയാറ്റൂർ", &["Malayattoor"]),
    ("കറുകുറ്റി", &["Karukutty"]),
    ("പൈങ്ങോട്ടൂർ", &["Paingottoor", "Paingotoor"]),
    ("തിരുമാറാടി", &["Thirumarady"]),
    ("ചിലവന്നൂർ", &["Chilavannoor", "Chilavannur"]),
    ("പുത്തൻകുരിശ്", &["Puthencruz"]),
    ("വാഴക്കുളം", &["Vazhakulam"]),
    ("ഫോർട്ട്കൊച്ചി", &["Fort Kochi", "Fort Cochin"]),
    // Common project words.
    ("റോഡ്", &["road"]),
    ("പാത", &["road"]),
    ("പാലം", &["bridge"]),
    ("മേൽപ്പാലം", &["flyover", "over bridge"]),
    ("ഫ്ലൈഓവർ", &["flyover"]),
    ("ബൈപാസ്", &["bypass"]),
    ("ബൈപ്പാസ്", &["bypass"]),
    ("ജംഗ്ഷൻ", &["junction"]),
    ("ഹൈവേ", &["highway"]),
    ("സ്കൂൾ", &["school"]),
    ("വിദ്യാലയം", &["school"]),
    ("കോളേജ്", &["college"]),
    ("കോളജ്", &["college"]),
    ("സർവകലാശാല", &["university"]),
    ("പോളിടെക്നിക്", &["polytechnic"]),
    ("ഹോസ്റ്റൽ", &["hostel"]),
    ("ആശുപത്രി", &["hospital"]),
    ("മെഡിക്കൽ", &["medical"]),
    ("കുടിവെള്ളം", &["water supply", "drinking water", "WSS"]),
    ("ജലവിതരണം", &["water supply", "WSS"]),
    ("കെട്ടിടം", &["building"]),
    ("സ്റ്റേഡിയം", &["stadium"]),
    ("മാർക്കറ്റ്", &["market"]),
    ("ചന്ത", &["market"]),
    ("കനാൽ", &["canal"]),
    ("തോട്", &["thodu", "canal"]),
    ("കടൽഭിത്തി", &["sea wall"]),
    ("തുറമുഖം", &["harbour"]),
    ("ഹാർബർ", &["harbour"]),
    ("ജെട്ടി", &["jetty"]),
    ("മെട്രോ", &["metro"]),
    ("തീരദേശ", &["coastal"]),
    ("മലയോര", &["hill highway"]),
    ("കോടതി", &["court"]),
    ("പോലീസ്", &["police"]),
    ("താലൂക്ക്", &["taluk"]),
    ("പഞ്ചായത്ത്", &["panchayat", "panchayath"]),
    ("മുനിസിപ്പാലിറ്റി", &["municipality"]),
    ("ലൈബ്രറി", &["library"]),
    ("തിയേറ്റർ", &["theatre", "theater"]),
];

/// Case endings to try removing, longest first, and what to put back in their place.
const ENDINGS: &[(&str, &str)] = &[
    ("ത്തിന്റെ", "ം"),
    ("ത്തിലെ", "ം"),
    ("ത്തിൽ", "ം"),
    ("ത്തെ", "ം"),
    ("യിലെ", ""),
    ("യിൽ", ""),
    ("യുടെ", ""),
    ("ിന്റെ", "്"),
    ("ിലെ", "്"),
    ("ിൽ", "്"),
    ("ുകൾ", "്"),
    ("കൾ", ""),
];

pub fn has_malayalam(s: &str) -> bool {
    s.chars().any(|c| ('\u{0d00}'..='\u{0d7f}').contains(&c))
}

/// One group of alternatives per word of the query. A Latin word is its own only alternative.
pub fn expand(query: &str) -> Vec<Vec<String>> {
    query
        .split_whitespace()
        .map(|word| {
            if !has_malayalam(word) {
                return vec![word.to_string()];
            }
            let word = word.trim_matches(|c: char| !c.is_alphanumeric() && !('\u{0d00}'..='\u{0d7f}').contains(&c));
            if let Some(found) = lookup(word) {
                return found;
            }
            for (ending, restore) in ENDINGS {
                if let Some(stem) = word.strip_suffix(ending) {
                    let stem = format!("{stem}{restore}");
                    // Before an ending the last consonant is often doubled: കാക്കനാട് + ഇലെ = കാക്കനാട്ടിലെ.
                    if let Some(found) = lookup(&stem).or_else(|| undoubled(&stem).and_then(|single| lookup(&single))) {
                        return found;
                    }
                }
            }
            spellings(word)
        })
        .filter(|alternatives| !alternatives.is_empty())
        .collect()
}

/// "കാക്കനാട്ട്" becomes "കാക്കനാട്": a final doubled consonant made single.
fn undoubled(stem: &str) -> Option<String> {
    let chars: Vec<char> = stem.chars().collect();
    match chars.as_slice() {
        [head @ .., a, '്', b, '്'] if a == b => Some(head.iter().chain([a, &'്']).collect()),
        _ => None,
    }
}

fn lookup(word: &str) -> Option<Vec<String>> {
    if let Some((_, english)) = WORDS.iter().find(|(ml, _)| *ml == word) {
        return Some(english.iter().map(|s| s.to_string()).collect());
    }
    CONSTITUENCIES.iter().find(|(_, ml, _)| *ml == word).map(|(name, _, _)| vec![name.to_string()])
}

/// A Malayalam word spelled in Latin letters: as it sounds, and in the plainer way many
/// official spellings use ("th" as "t", long vowels short, doubled letters single).
fn spellings(word: &str) -> Vec<String> {
    let full = romanise(word);
    if full.chars().count() < 3 {
        return Vec::new();
    }
    let mut plain = full.replace("ee", "i").replace("oo", "u").replace("aa", "a");
    let mut undoubled = String::with_capacity(plain.len());
    for c in plain.chars() {
        if undoubled.chars().last() != Some(c) {
            undoubled.push(c);
        }
    }
    plain = undoubled;
    let mut out = vec![full.clone()];
    for alternative in [plain.clone(), plain.replace("th", "t")] {
        if alternative.chars().count() >= 3 && !out.contains(&alternative) {
            out.push(alternative);
        }
    }
    out
}

fn romanise(word: &str) -> String {
    let mut out = String::new();
    let mut pending_a = false;
    for c in word.chars() {
        let consonant = match c {
            'ക' => "k", 'ഖ' => "kh", 'ഗ' => "g", 'ഘ' => "gh", 'ങ' => "ng",
            'ച' => "ch", 'ഛ' => "chh", 'ജ' => "j", 'ഝ' => "jh", 'ഞ' => "nj",
            'ട' => "t", 'ഠ' => "th", 'ഡ' => "d", 'ഢ' => "dh", 'ണ' => "n",
            'ത' => "th", 'ഥ' => "th", 'ദ' => "d", 'ധ' => "dh", 'ന' => "n",
            'പ' => "p", 'ഫ' => "ph", 'ബ' => "b", 'ഭ' => "bh", 'മ' => "m",
            'യ' => "y", 'ര' => "r", 'ല' => "l", 'വ' => "v", 'ശ' => "sh", 'ഷ' => "sh",
            'സ' => "s", 'ഹ' => "h", 'ള' => "l", 'ഴ' => "zh", 'റ' => "r",
            _ => "",
        };
        if !consonant.is_empty() {
            if pending_a {
                out.push('a');
            }
            out.push_str(consonant);
            pending_a = true;
            continue;
        }
        let (sound, keeps_a) = match c {
            'അ' => ("a", false), 'ആ' => ("aa", false), 'ഇ' => ("i", false), 'ഈ' => ("ee", false),
            'ഉ' => ("u", false), 'ഊ' => ("oo", false), 'ഋ' => ("ru", false), 'എ' | 'ഏ' => ("e", false),
            'ഐ' => ("ai", false), 'ഒ' | 'ഓ' => ("o", false), 'ഔ' => ("au", false),
            'ാ' => ("aa", false), 'ി' => ("i", false), 'ീ' => ("ee", false), 'ു' => ("u", false),
            'ൂ' => ("oo", false), 'ൃ' => ("ru", false), 'െ' | 'േ' => ("e", false), 'ൈ' => ("ai", false),
            'ൊ' | 'ോ' => ("o", false), 'ൌ' | 'ൗ' => ("au", false),
            '്' => ("", false),
            'ം' => ("m", true),
            'ൻ' | 'ൺ' => ("n", true), 'ർ' => ("r", true), 'ൽ' | 'ൾ' => ("l", true), 'ൿ' => ("k", true),
            _ => continue,
        };
        if keeps_a && pending_a {
            out.push('a');
        }
        out.push_str(sound);
        pending_a = false;
    }
    if pending_a {
        out.push('a');
    }
    out
}

/// A search that found nothing, as it is kept for review: lower-case, single-spaced, at most
/// 60 characters. `None` for text that is too short to mean anything or that looks like
/// something personal (an email address or a phone number), which is never kept.
pub fn miss_key(query: &str) -> Option<String> {
    let key: String = query.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase().chars().take(60).collect();
    let digits = key.chars().filter(char::is_ascii_digit).count();
    (key.chars().count() >= 2 && !key.contains('@') && digits < 7).then_some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(word: &str) -> Vec<String> {
        expand(word).into_iter().next().unwrap_or_default()
    }

    #[test]
    fn latin_words_pass_through() {
        assert_eq!(expand("Aluva  bridge"), [vec!["Aluva".to_string()], vec!["bridge".to_string()]]);
        assert!(!has_malayalam("Aluva"));
        assert!(has_malayalam("ആലുവ"));
    }

    #[test]
    fn known_places_and_words_come_from_the_dictionary() {
        assert_eq!(one("ആലുവ"), ["Aluva"]);
        assert_eq!(one("തൃപ്പൂണിത്തുറ"), ["Thripunithura"]);
        assert_eq!(one("കൊച്ചി"), ["Kochi", "Cochin"]);
        assert_eq!(one("പാലം"), ["bridge"]);
        assert_eq!(one("ഒല്ലൂർ"), ["Ollur"], "every constituency in the state is known");
        assert_eq!(expand("ആലുവ റോഡ്"), [vec!["Aluva".to_string()], vec!["road".to_string()]]);
    }

    #[test]
    fn case_endings_are_removed() {
        assert_eq!(one("ആലുവയിലെ"), ["Aluva"]);
        assert_eq!(one("എറണാകുളത്തെ"), ["Ernakulam"]);
        assert_eq!(one("കോട്ടയത്തിലെ"), ["Kottayam"]);
        assert_eq!(one("കാക്കനാടിലെ"), ["Kakkanad"]);
        assert_eq!(one("കാക്കനാട്ടിലെ"), ["Kakkanad"]);
        assert_eq!(one("പാലക്കാട്ടിൽ"), ["Palakkad"]);
        assert_eq!(one("റോഡുകൾ"), ["road"]);
    }

    #[test]
    fn unknown_words_are_spelled_out_in_more_than_one_way() {
        assert_eq!(one("വാളകം"), ["vaalakam", "valakam"]);
        assert_eq!(one("മൂക്കന്നൂർ"), ["mookkannoor", "mukanur"]);
        assert_eq!(one("തുറവൂർ"), ["thuravoor", "thuravur", "turavur"]);
        assert!(one("അ").is_empty(), "too short to search for");
    }

    #[test]
    fn missed_searches_are_tidied_and_personal_text_is_dropped() {
        assert_eq!(miss_key("  Aluva   BRIDGE "), Some("aluva bridge".to_string()));
        assert_eq!(miss_key("ആലുവ പാലം"), Some("ആലുവ പാലം".to_string()));
        assert_eq!(miss_key("a"), None);
        assert_eq!(miss_key("someone@example.com"), None);
        assert_eq!(miss_key("call 9876543210"), None);
        assert_eq!(miss_key("NH 66 km 120"), Some("nh 66 km 120".to_string()));
        assert_eq!(miss_key(&"x".repeat(200)).map(|k| k.len()), Some(60));
    }
}
