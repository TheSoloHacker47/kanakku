//! Readable project titles. KIIFB titles often start with filing prefixes and come in capitals.
//! The original is always kept and shown; this only shapes the headline.

const SMALL_WORDS: &[&str] = &["a", "an", "and", "at", "by", "for", "from", "in", "of", "on", "the", "to", "with"];

const ACRONYMS: &[&str] = &[
    "AC", "CCTV", "CCU", "CH", "CHC", "CUSAT", "CWPM", "DI", "FHC", "GH", "GHS", "GHSS", "GLPS", "GMHSS", "GP", "GUPS", "GVHSS", "HS",
    "HSS", "HT", "HVAC", "ICU", "IHRD", "II", "III", "IT", "ITI", "IV", "KIIDC", "KIIFB", "KILA", "KM", "KMRL", "KRFB", "KSEB",
    "KWA", "LA", "LED", "LP", "LT", "MCH", "MGPS", "MLD", "NH", "OHSR", "OP", "PHC", "PWD", "RBDCK", "RCC", "ROB", "RUB", "SC", "SH",
    "ST", "STP", "TH", "THQH", "UPS", "VHSS", "VI", "VII", "WSS", "WTP",
];

/// The title to show as a headline.
pub fn display_title(raw: &str) -> String {
    let stripped = strip_filing_prefix(raw.trim());
    let collapsed = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    let letters = collapsed.chars().filter(|c| c.is_alphabetic()).count();
    let capitals = collapsed.chars().filter(|c| c.is_uppercase()).count();
    // Mostly capitals: calm the shouted words and leave the rest as written.
    let shouting = letters > 0 && capitals * 10 >= letters * 7;
    let title = if shouting && collapsed.split(' ').count() > 2 { title_case(&collapsed) } else { collapsed };
    if title.is_empty() {
        raw.trim().to_string()
    } else {
        upper_first(&title)
    }
}

/// Drops leading "KIIFB -", "KILA-KIIFB-CL-07-" and "PWD015-05 -" style prefixes.
fn strip_filing_prefix(mut s: &str) -> &str {
    loop {
        let before = s;
        for org in ["KIIFB", "KIFB", "KILA"] {
            if let Some(rest) = strip_token(s, org) {
                s = rest;
            }
        }
        if let Some(rest) = strip_code(s) {
            s = rest;
        }
        if s == before {
            return s;
        }
    }
}

/// Strips `token` when it is followed by a separator.
fn strip_token<'a>(s: &'a str, token: &str) -> Option<&'a str> {
    let head = s.get(..token.len())?;
    if !head.eq_ignore_ascii_case(token) {
        return None;
    }
    let rest = &s[token.len()..];
    let trimmed = rest.trim_start_matches([' ', '-', '–', ':']);
    (trimmed.len() < rest.len() && !trimmed.is_empty()).then_some(trimmed)
}

/// Strips a leading filing code such as `CL-07-` or `WRD005-125-` when a dash follows it.
fn strip_code(s: &str) -> Option<&str> {
    let letters = s.bytes().take_while(u8::is_ascii_uppercase).count();
    if !(2..=4).contains(&letters) {
        return None;
    }
    let mut i = letters + s[letters..].bytes().take_while(u8::is_ascii_digit).count();
    let mut groups = 0;
    while s[i..].starts_with('-') {
        let digits = s[i + 1..].bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            break;
        }
        i += 1 + digits;
        groups += 1;
    }
    if groups == 0 {
        return None;
    }
    let rest = &s[i..];
    let trimmed = rest.trim_start();
    let trimmed = trimmed.strip_prefix(['-', '–', ':'])?.trim_start_matches([' ', '-', '–', ':']);
    (!trimmed.is_empty()).then_some(trimmed)
}

fn title_case(s: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    words
        .iter()
        .enumerate()
        .map(|(i, word)| {
            // "UP" is Upper Primary before "School" and an ordinary word everywhere else.
            let school_next = words.get(i + 1).is_some_and(|next| next.to_lowercase().starts_with("school"));
            if *word == "UP" && school_next {
                return (*word).to_string();
            }
            word.split('-').map(|part| case_word(part, i == 0)).collect::<Vec<_>>().join("-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn case_word(word: &str, first: bool) -> String {
    let core: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
    let written_normally = core.chars().any(char::is_lowercase);
    if core.is_empty() || written_normally || core.chars().any(|c| c.is_ascii_digit()) || ACRONYMS.contains(&core.as_str()) {
        return word.to_string();
    }
    let lower = word.to_lowercase();
    if !first && SMALL_WORDS.contains(&core.to_lowercase().as_str()) {
        return lower;
    }
    // Capitalise the first letter, which may sit behind a bracket or quote.
    let mut out = String::with_capacity(lower.len());
    let mut done = false;
    for c in lower.chars() {
        if !done && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            done = true;
        } else {
            out.push(c);
        }
    }
    out
}

fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::display_title as t;

    #[test]
    fn strips_filing_prefixes() {
        assert_eq!(
            t("KILA- KIIFB- CL-07-Construction of School Building to GOVT GHSS MOOKKANNOOR in Mookkannoor GP Ernakulam Dt"),
            "Construction of School Building to GOVT GHSS MOOKKANNOOR in Mookkannoor GP Ernakulam Dt"
        );
        assert_eq!(
            t("KIIFB-WRD005-125-Replacement of transmission mains-- Improvements of water supply"),
            "Replacement of transmission mains-- Improvements of water supply"
        );
        assert_eq!(t("KIIFB - AUGMENTATION OF WSS TO ANGAMALY CONSTITUENCY - PART 1"), "Augmentation of WSS to Angamaly Constituency - Part 1");
        assert_eq!(t("KILA-KIFB-CL-07-Construction of School Building"), "Construction of School Building");
        assert_eq!(t("PWD003-16: Construction of Muvattupuzha Town Bypass Road"), "Construction of Muvattupuzha Town Bypass Road");
    }

    #[test]
    fn calms_capitals_but_keeps_acronyms_and_numbers() {
        assert_eq!(
            t("PWD015-05 - DEVELOPMENT OF MUVATTUPUZHA TOWN PORTION CH 0/000 TO CH 1/850 POST OFFICE JUNCTION TO PETTA ROAD - Work Contract"),
            "Development of Muvattupuzha Town Portion CH 0/000 to CH 1/850 Post Office Junction to Petta Road - Work Contract"
        );
        assert_eq!(t("ELECTRICAL WORKS OF CCU AT MCH KALAMASSERRY"), "Electrical Works of CCU at MCH Kalamasserry");
        assert_eq!(
            t("SETTINGUP OF DIALYSIS CENTRE-ELECTRICAL WORKS AT GH MUVATTUPUZHA"),
            "Settingup of Dialysis Centre-Electrical Works at GH Muvattupuzha"
        );
        assert_eq!(t("SETTING UP OF DIALYSIS CENTRE -CIVIL WORKS AT GH MUVATTUPUZHA"), "Setting Up of Dialysis Centre -Civil Works at GH Muvattupuzha");
        assert_eq!(t("CONSTRUCTION OF BUILDING FOR GOVT UP SCHOOL KOOTHATTUKULAM"), "Construction of Building for Govt UP School Koothattukulam");
        assert_eq!(
            t("CONSTRUCTION OF TANK BUND ROAD BRIDGE IN CHILAVANNUR CANAL"),
            "Construction of Tank Bund Road Bridge in Chilavannur Canal"
        );
    }

    #[test]
    fn leaves_ordinary_titles_alone() {
        for title in ["68 Bridges", "Coastal Highway", "Laser Ablation/LIBS", "GHSS Puthiyakavu", "Development of 43 roads", "182 Roads"] {
            assert_eq!(t(title), title);
        }
        assert_eq!(t("  Hill   Highway "), "Hill Highway");
        assert_eq!(t("KIIFB"), "KIIFB");
        assert_eq!(t(""), "");
        assert_eq!(t("IT PARK"), "IT PARK", "two words are not worth recasing");
    }
}
