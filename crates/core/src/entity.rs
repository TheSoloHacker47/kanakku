//! One key per contractor and per implementing agency, however a source spells the name.
//!
//! KIIFB's dashboard runs contractor names together without spaces and sometimes appends an
//! address; PWD writes them with titles and in capitals; agencies appear under their full name
//! in one place and an abbreviation in another. The keys here make those comparable. They are
//! deliberately simple: two different people with the same name share a key, and pages say so.

const TITLES: [&str; 8] = ["shri", "sri", "smt", "mr", "mrs", "ms", "messrs", "m/s"];
const COMPANY_ENDINGS: [&str; 6] = ["privatelimited", "pvtltd", "limited", "ltd", "private", "pvt"];

/// A contractor's key: the name without title, address, punctuation, case or company ending.
pub fn contractor_key(name: &str) -> String {
    // Anything after the first comma is an address.
    let name = name.split(',').next().unwrap_or("").to_lowercase();
    let name = name.trim().trim_start_matches("m/s.").trim_start_matches("m/s");
    let mut key: String = name
        .split(|c: char| c.is_whitespace() || c == '.')
        .filter(|word| !word.is_empty() && !TITLES.contains(word))
        .flat_map(|word| word.chars().filter(|c| c.is_alphanumeric()))
        .collect();
    strip_company_endings(&mut key);
    // A name with no letters ("0", "-") is a placeholder a source printed, not a contractor.
    if !key.chars().any(char::is_alphabetic) {
        key.clear();
    }
    key
}

/// A contractor's name for display: address and title dropped, capitals calmed, and words that
/// KIIFB ran together separated again where their capitals show the joins.
pub fn contractor_display(name: &str) -> String {
    const DROP: [&str; 6] = ["shri", "sri", "smt", "mr", "mrs", "ms"];
    let name = name.split(',').next().unwrap_or("").trim();
    let spaced = if name.contains(' ') { name.to_string() } else { split_joined(name) };
    let words: Vec<&str> =
        spaced.split_whitespace().filter(|word| !DROP.contains(&word.trim_end_matches('.').to_lowercase().as_str())).collect();
    let letters = words.iter().flat_map(|w| w.chars()).filter(|c| c.is_alphabetic()).count();
    let capitals = words.iter().flat_map(|w| w.chars()).filter(|c| c.is_uppercase()).count();
    let shouting = letters > 3 && capitals * 10 >= letters * 8;
    words
        .iter()
        .map(|word| {
            if !shouting || word.chars().filter(|c| c.is_alphabetic()).count() <= 2 || word.eq_ignore_ascii_case("M/s") {
                (*word).to_string()
            } else {
                capitalise(word)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "MarymathaInfrastructurePrivateLimited." becomes "Marymatha Infrastructure Private Limited.";
/// "Sri.K.I.Paulose" becomes "Sri. K. I. Paulose". A name all in capitals has no joins to find.
fn split_joined(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 8);
    let chars: Vec<char> = name.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        let previous = i.checked_sub(1).map(|j| chars[j]);
        let lower_to_upper = c.is_uppercase() && previous.is_some_and(|p| p.is_lowercase());
        let after_dot = previous == Some('.') && c.is_alphabetic();
        if lower_to_upper || after_dot {
            out.push(' ');
        }
        out.push(*c);
    }
    out.replace("M/s", "M/s ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capitalise(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut first = true;
    for c in word.chars() {
        if first && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            first = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn strip_company_endings(key: &mut String) {
    while let Some(ending) = COMPANY_ENDINGS.iter().find(|e| key.len() > e.len() + 2 && key.ends_with(*e)) {
        key.truncate(key.len() - ending.len());
    }
}

/// Agencies the sources name both in full and by abbreviation: `(abbreviation, key, display name)`.
const AGENCIES: &[(&str, &str, &str)] = &[
    ("krfb", "keralaroadfundboard", "Kerala Road Fund Board (KRFB)"),
    ("rbdck", "roadsandbridgesdevelopmentcorporationofkerala", "Roads and Bridges Development Corporation of Kerala (RBDCK)"),
    ("kmrl", "kochimetrorail", "Kochi Metro Rail Limited (KMRL)"),
    ("kiidc", "keralairrigationinfrastructuredevelopmentcorporation", "Kerala Irrigation Infrastructure Development Corporation (KIIDC)"),
    ("kite", "keralainfrastructureandtechnologyforeducation", "Kerala Infrastructure and Technology for Education (KITE)"),
    ("kwa", "keralawaterauthority", "Kerala Water Authority (KWA)"),
    ("kila", "keralainstituteoflocaladministration", "Kerala Institute of Local Administration (KILA)"),
    ("kmscl", "keralamedicalservicescorporation", "Kerala Medical Services Corporation (KMSCL)"),
    ("kinfra", "keralaindustrialinfrastructuredevelopmentcorporation", "Kerala Industrial Infrastructure Development Corporation (KINFRA)"),
    ("ksitil", "keralastateinformationtechnologyinfrastructure", "Kerala State Information Technology Infrastructure Ltd (KSITIL)"),
    ("cusat", "cochinuniversityofscienceandtechnology", "Cochin University of Science and Technology (CUSAT)"),
    ("impact", "investmentinmunicipalandpanchayatassetcreationfortransformationkerala", "IMPACT Kerala Ltd"),
    ("gcda", "greatercochindevelopmentauthority", "Greater Cochin Development Authority (GCDA)"),
];

const ACRONYMS: [&str; 12] = ["INKEL", "KITCO", "WAPCOS", "HLL", "KSEBL", "KSEB", "USLR", "KINFRA", "KSRTC", "LLP", "PWD", "IT"];

/// An agency's key: brackets, punctuation, case and company ending removed, abbreviations resolved.
pub fn agency_key(name: &str) -> String {
    let mut plain = String::with_capacity(name.len());
    let mut depth = 0u32;
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let mut key: String = plain.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    strip_company_endings(&mut key);
    match AGENCIES.iter().find(|(abbreviation, full, _)| key == *abbreviation || key == *full) {
        Some((_, full, _)) => (*full).to_string(),
        None => key,
    }
}

/// An agency's name for display: the known full name, or the published one with capitals calmed.
pub fn agency_display(name: &str) -> String {
    let key = agency_key(name);
    if let Some((_, _, display)) = AGENCIES.iter().find(|(_, full, _)| key == *full) {
        return (*display).to_string();
    }
    let letters = name.chars().filter(|c| c.is_alphabetic()).count();
    let capitals = name.chars().filter(|c| c.is_uppercase()).count();
    // Mixed case is left alone, and so is a single short word in capitals: it is an abbreviation.
    if letters < 4 || capitals * 10 < letters * 8 || (letters <= 6 && !name.trim().contains(' ')) {
        return name.trim().to_string();
    }
    name.split_whitespace()
        .map(|word| {
            let bare = word.trim_matches(|c: char| !c.is_alphanumeric());
            if ACRONYMS.contains(&bare) {
                word.to_string()
            } else if ["OF", "AND", "FOR", "IN", "THE"].contains(&bare) {
                word.to_lowercase()
            } else {
                capitalise(word)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_key_for_every_spelling_of_a_contractor() {
        let key = contractor_key("URALUNGALLABOURCONTRACTCO-OPERATIVESOCIETYLIMITED");
        assert_eq!(key, "uralungallabourcontractcooperativesociety");
        assert_eq!(contractor_key("M/s.UralungalLabourContractCo-operative-SocietyLtd"), key);
        assert_eq!(contractor_key("M/s Uralungal Labour Contract Co-operative Society Ltd."), key);

        assert_eq!(contractor_key("Sri.K.I.Paulose"), contractor_key("K.I.Paulose"));
        assert_eq!(contractor_key("Shri. P.V. Stephan"), contractor_key("P V STEPHAN"));
        assert_eq!(
            contractor_key("M/sRajeshMathewandCompany,CherumattathilPlaza,ArakuzhaRoadP.O.Junction,Muvattupuzha-686661"),
            contractor_key("RAJESHMATHEWANDCOMPANY")
        );
        assert_ne!(contractor_key("P V Stephan"), contractor_key("P V Stephen"));
        assert_eq!(contractor_key("0"), "", "a placeholder is nobody");
        assert_eq!(contractor_key(" - "), "");
    }

    #[test]
    fn an_address_never_reaches_the_key_or_the_display_name() {
        let raw = "RAJUCHACKO,ADUKUZHIYLHOUSE,SOUTHMARADYP.O,,MUVATTUPUZHA,Ernakulam,Kerala,686673";
        assert_eq!(contractor_key(raw), "rajuchacko");
        assert_eq!(contractor_display(raw), "Rajuchacko");
    }

    #[test]
    fn display_names_are_readable() {
        assert_eq!(contractor_display("MarymathaInfrastructurePrivateLimited."), "Marymatha Infrastructure Private Limited.");
        assert_eq!(contractor_display("Sri.K.I.Paulose"), "K. I. Paulose");
        assert_eq!(contractor_display("P V STEPHAN"), "P V Stephan");
        assert_eq!(contractor_display("Shri. A. ABDUL HAKKIM"), "A. Abdul Hakkim");
        assert_eq!(contractor_display("Baiju A A"), "Baiju A A");
        assert_eq!(contractor_display("M/s Chemparaky LCS"), "M/s Chemparaky LCS");
        assert_eq!(contractor_display("M/sRajeshMathewandCompany,Plaza"), "M/s Rajesh Mathewand Company");
    }

    #[test]
    fn agencies_resolve_abbreviations_and_endings() {
        let krfb = agency_key("KERALA ROAD FUND BOARD");
        assert_eq!(agency_key("KRFB"), krfb);
        assert_eq!(agency_key("KERALA ROAD FUND BOARD (KRFB)"), krfb);
        assert_eq!(agency_key("KMRL"), agency_key("KOCHI METRO RAIL LIMITED"));
        assert_eq!(agency_key("Kerala Industrial Infrastructure Development Corporation [KINFRA]"), agency_key("KINFRA"));
        assert_eq!(agency_key("KERALA STATE CONSTRUCTION CORPORATION LTD."), "keralastateconstructioncorporation");
        assert_ne!(agency_key("INKEL LIMITED"), agency_key("KITCO LTD"));
    }

    #[test]
    fn agency_names_display_calmly() {
        assert_eq!(agency_display("KRFB"), "Kerala Road Fund Board (KRFB)");
        assert_eq!(agency_display("INKEL LIMITED"), "INKEL Limited");
        assert_eq!(agency_display("KERALA STATE COASTAL AREA DEVELOPMENT CORPORATION LIMITED"), "Kerala State Coastal Area Development Corporation Limited");
        assert_eq!(agency_display("Kerala State Housing Board"), "Kerala State Housing Board");
        assert_eq!(agency_display("SPORTS KERALA FOUNDATION"), "Sports Kerala Foundation");
        assert_eq!(agency_display("PWDD"), "PWDD");
    }
}
