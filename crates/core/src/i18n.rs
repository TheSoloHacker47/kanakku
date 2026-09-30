//! UI strings. Malayalam is the default language; English is the toggle.

use serde_json::Value;

use crate::flags::FlagKind;
use crate::Date;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ml,
    En,
}

impl Lang {
    pub fn pick<'a>(self, ml: &'a str, en: &'a str) -> &'a str {
        match self {
            Lang::Ml => ml,
            Lang::En => en,
        }
    }

    pub fn code(self) -> &'static str {
        self.pick("ml", "en")
    }

    /// URL prefix: Malayalam lives at the root, English under `/en`.
    pub fn prefix(self) -> &'static str {
        self.pick("", "/en")
    }

    pub fn other(self) -> Lang {
        match self {
            Lang::Ml => Lang::En,
            Lang::En => Lang::Ml,
        }
    }

    pub fn t(self) -> &'static Strings {
        match self {
            Lang::Ml => &ML,
            Lang::En => &EN,
        }
    }
}

pub struct Strings {
    pub site_name: &'static str,
    pub tagline: &'static str,
    pub lang_name: &'static str,
    pub nav_projects: &'static str,
    pub nav_map: &'static str,
    pub nav_methodology: &'static str,
    pub nav_data: &'static str,
    pub skip_to_content: &'static str,

    pub list_title: &'static str,
    pub list_intro: &'static str,
    pub search_label: &'static str,
    pub search_placeholder: &'static str,
    pub filter_department: &'static str,
    pub filter_constituency: &'static str,
    pub filter_status: &'static str,
    pub filter_flag: &'static str,
    pub filter_all: &'static str,
    pub filter_any_flag: &'static str,
    pub filters: &'static str,
    pub apply: &'static str,
    pub clear: &'static str,
    pub projects_found: &'static str,
    pub no_results: &'static str,
    pub previous: &'static str,
    pub next: &'static str,
    pub page: &'static str,
    pub works_total: &'static str,

    pub code: &'static str,
    pub department: &'static str,
    pub executing_agency: &'static str,
    pub constituency: &'static str,
    pub mla: &'static str,
    pub status: &'static str,
    pub money_trail: &'static str,
    pub estimated_amount: &'static str,
    pub expenditure: &'static str,
    pub spent_share: &'static str,
    pub works: &'static str,
    pub work: &'static str,
    pub contractor: &'static str,
    pub as_amount: &'static str,
    pub fs_amount: &'static str,
    pub ts_amount: &'static str,
    pub tender_amount: &'static str,
    pub loa_amount: &'static str,
    pub contract_amount: &'static str,
    pub paid_amount: &'static str,
    pub paid_contractor: &'static str,
    pub timeline: &'static str,
    pub scheduled_start: &'static str,
    pub scheduled_end: &'static str,
    pub physical_progress: &'static str,
    pub financial_progress: &'static str,
    pub progress_note: &'static str,
    pub flags: &'static str,
    pub no_flags: &'static str,
    pub rule_version: &'static str,
    pub gaps: &'static str,
    pub gaps_intro: &'static str,
    pub no_gaps: &'static str,
    pub location: &'static str,
    pub open_in_maps: &'static str,
    pub sources: &'static str,
    pub source_kiifb: &'static str,
    pub retrieved: &'static str,
    pub download_snapshot: &'static str,
    pub first_seen: &'static str,
    pub changes: &'static str,
    pub not_reported: &'static str,
    pub back_to_list: &'static str,
    pub not_found_title: &'static str,
    pub not_found_body: &'static str,

    pub map_title: &'static str,
    pub map_intro: &'static str,
    pub map_legend_flagged: &'static str,
    pub map_legend_clear: &'static str,
    pub map_loading: &'static str,

    pub methodology_title: &'static str,
    pub data_title: &'static str,
    pub footer_principle: &'static str,
    pub footer_source: &'static str,
    pub footer_corrections: &'static str,
}

pub static ML: Strings = Strings {
    site_name: "കണക്ക്",
    tagline: "പൊതുപദ്ധതികളുടെ കണക്ക്, രേഖകളോടെ",
    lang_name: "മലയാളം",
    nav_projects: "പദ്ധതികൾ",
    nav_map: "ഭൂപടം",
    nav_methodology: "രീതി",
    nav_data: "ഡാറ്റ",
    skip_to_content: "ഉള്ളടക്കത്തിലേക്ക്",

    list_title: "കേരളത്തിലെ കിഫ്ബി പദ്ധതികൾ",
    list_intro: "അനുവദിച്ച തുക, കരാറുകാരൻ, നൽകിയ തുക, പുരോഗതി. ഓരോ സംഖ്യയും അതിന്റെ ഉറവിടത്തോടൊപ്പം.",
    search_label: "തിരയുക",
    search_placeholder: "പദ്ധതി, കോഡ്, കരാറുകാരൻ (ഇംഗ്ലീഷിൽ)",
    filter_department: "വകുപ്പ്",
    filter_constituency: "നിയമസഭാ മണ്ഡലം",
    filter_status: "നില",
    filter_flag: "സൂചന",
    filter_all: "എല്ലാം",
    filter_any_flag: "ഏതെങ്കിലും സൂചനയുള്ളവ",
    filters: "അരിപ്പകൾ",
    apply: "കാണിക്കുക",
    clear: "മായ്ക്കുക",
    projects_found: "പദ്ധതികൾ",
    no_results: "ഈ തിരച്ചിലിന് യോജിച്ച പദ്ധതികളില്ല.",
    previous: "മുൻപത്തെ",
    next: "അടുത്തത്",
    page: "താൾ",
    works_total: "പ്രവൃത്തികളുടെ ആകെ തുക",

    code: "പദ്ധതി കോഡ്",
    department: "വകുപ്പ്",
    executing_agency: "നിർവഹണ ഏജൻസി",
    constituency: "മണ്ഡലം",
    mla: "എം.എൽ.എ",
    status: "ഔദ്യോഗിക നില",
    money_trail: "പണത്തിന്റെ വഴി",
    estimated_amount: "കണക്കാക്കിയ തുക",
    expenditure: "രേഖപ്പെടുത്തിയ ചെലവ്",
    spent_share: "കണക്കാക്കിയ തുകയുടെ",
    works: "പ്രവൃത്തികൾ",
    work: "പ്രവൃത്തി",
    contractor: "കരാറുകാരൻ",
    as_amount: "ഭരണാനുമതി (AS)",
    fs_amount: "സാമ്പത്തികാനുമതി (FS)",
    ts_amount: "സാങ്കേതികാനുമതി (TS)",
    tender_amount: "ടെൻഡർ തുക",
    loa_amount: "സ്വീകാര്യതാപത്ര തുക (LOA)",
    contract_amount: "കരാർ തുക",
    paid_amount: "നൽകിയ തുക",
    paid_contractor: "കരാറുകാരന് നൽകിയത്",
    timeline: "സമയക്രമം",
    scheduled_start: "നിശ്ചയിച്ച തുടക്കം",
    scheduled_end: "നിശ്ചയിച്ച പൂർത്തീകരണം",
    physical_progress: "ഭൗതിക പുരോഗതി",
    financial_progress: "സാമ്പത്തിക പുരോഗതി",
    progress_note: "നിലവിലെ പുരോഗതി",
    flags: "സൂചനകൾ",
    no_flags: "നിലവിലെ നിയമങ്ങൾ പ്രകാരം ഈ പദ്ധതിക്ക് സൂചനകളൊന്നുമില്ല.",
    rule_version: "നിയമ പതിപ്പ്",
    gaps: "ഇനിയും അറിയാത്തത്",
    gaps_intro: "കിഫ്ബി ഡാഷ്ബോർഡിൽ ഈ വിവരങ്ങൾ പ്രസിദ്ധീകരിച്ചിട്ടില്ല:",
    no_gaps: "ഞങ്ങൾ പരിശോധിക്കുന്ന എല്ലാ വിവരങ്ങളും ഡാഷ്ബോർഡിലുണ്ട്.",
    location: "സ്ഥലം",
    open_in_maps: "ഭൂപടത്തിൽ തുറക്കുക",
    sources: "ഉറവിടം",
    source_kiifb: "കിഫ്ബി സംയോജിത ഡാഷ്ബോർഡ്",
    retrieved: "ശേഖരിച്ച തീയതി",
    download_snapshot: "ഞങ്ങൾ സൂക്ഷിച്ച പകർപ്പ് ഡൗൺലോഡ് ചെയ്യുക",
    first_seen: "ആദ്യം രേഖപ്പെടുത്തിയത്",
    changes: "രേഖപ്പെടുത്തിയ മാറ്റങ്ങൾ",
    not_reported: "പ്രസിദ്ധീകരിച്ചിട്ടില്ല",
    back_to_list: "എല്ലാ പദ്ധതികളും",
    not_found_title: "ഈ താൾ കണ്ടെത്താനായില്ല",
    not_found_body: "വിലാസം പരിശോധിക്കുക, അല്ലെങ്കിൽ പദ്ധതികളുടെ പട്ടികയിൽ തിരയുക.",

    map_title: "ഭൂപടം",
    map_intro: "കിഫ്ബി രേഖപ്പെടുത്തിയ സ്ഥാനങ്ങളാണ് ഇവ. ചിലത് കൃത്യമല്ലാതിരിക്കാം.",
    map_legend_flagged: "സൂചനയുണ്ട്",
    map_legend_clear: "സൂചനയില്ല",
    map_loading: "ഭൂപടം ലോഡ് ചെയ്യുന്നു…",

    methodology_title: "രീതിശാസ്ത്രം",
    data_title: "തുറന്ന ഡാറ്റ",
    footer_principle: "വസ്തുതകളും സൂചനകളും മാത്രം; ആരോപണങ്ങളില്ല. എല്ലാ കക്ഷികൾക്കും ഒരേ നിയമം.",
    footer_source: "ഡാറ്റ: കിഫ്ബി സംയോജിത ഡാഷ്ബോർഡ് (gis.kiifb.org). അവസാനം ശേഖരിച്ചത്:",
    footer_corrections: "തെറ്റ് കണ്ടാൽ അറിയിക്കുക; ഞങ്ങൾ തിരുത്തും.",
};

pub static EN: Strings = Strings {
    site_name: "Kanakku",
    tagline: "Public project accounts, with the documents",
    lang_name: "English",
    nav_projects: "Projects",
    nav_map: "Map",
    nav_methodology: "Method",
    nav_data: "Data",
    skip_to_content: "Skip to content",

    list_title: "KIIFB projects in Kerala",
    list_intro: "What was sanctioned, who got the contract, what has been paid, and how far the work has come. Every number links to its source.",
    search_label: "Search",
    search_placeholder: "Project, code or contractor",
    filter_department: "Department",
    filter_constituency: "Assembly constituency",
    filter_status: "Status",
    filter_flag: "Flag",
    filter_all: "All",
    filter_any_flag: "Any flag",
    filters: "Filters",
    apply: "Show",
    clear: "Clear",
    projects_found: "projects",
    no_results: "No projects match this search.",
    previous: "Previous",
    next: "Next",
    page: "Page",
    works_total: "Total of works",

    code: "Project code",
    department: "Department",
    executing_agency: "Executing agency",
    constituency: "Constituency",
    mla: "MLA",
    status: "Official status",
    money_trail: "Money trail",
    estimated_amount: "Estimated amount",
    expenditure: "Expenditure reported",
    spent_share: "of the estimated amount",
    works: "Works",
    work: "Work",
    contractor: "Contractor",
    as_amount: "Administrative sanction (AS)",
    fs_amount: "Financial sanction (FS)",
    ts_amount: "Technical sanction (TS)",
    tender_amount: "Tender amount",
    loa_amount: "Letter of acceptance (LOA)",
    contract_amount: "Contract amount",
    paid_amount: "Amount paid",
    paid_contractor: "Paid to contractor",
    timeline: "Timeline",
    scheduled_start: "Scheduled start",
    scheduled_end: "Scheduled completion",
    physical_progress: "Physical progress",
    financial_progress: "Financial progress",
    progress_note: "Current progress",
    flags: "Flags",
    no_flags: "No flags for this project under the current rules.",
    rule_version: "Rule version",
    gaps: "What we don't know yet",
    gaps_intro: "The KIIFB dashboard does not publish these for this project:",
    no_gaps: "The dashboard publishes every field we check.",
    location: "Location",
    open_in_maps: "Open in maps",
    sources: "Source",
    source_kiifb: "KIIFB integrated dashboard",
    retrieved: "Retrieved",
    download_snapshot: "Download the copy we stored",
    first_seen: "First recorded",
    changes: "Recorded changes",
    not_reported: "Not published",
    back_to_list: "All projects",
    not_found_title: "Page not found",
    not_found_body: "Check the address, or search the project list.",

    map_title: "Map",
    map_intro: "These are the locations KIIFB records. Some may be inaccurate.",
    map_legend_flagged: "Has a flag",
    map_legend_clear: "No flag",
    map_loading: "Loading the map…",

    methodology_title: "Methodology",
    data_title: "Open data",
    footer_principle: "Facts and flags, never accusations. The same rules for every party.",
    footer_source: "Data: KIIFB integrated dashboard (gis.kiifb.org). Last retrieved:",
    footer_corrections: "Found a mistake? Tell us and we will correct it.",
};

pub fn flag_label(lang: Lang, kind: FlagKind) -> &'static str {
    match kind {
        FlagKind::Overdue => lang.pick("സമയപരിധി കഴിഞ്ഞു", "Overdue"),
        FlagKind::PaymentVsProgress => lang.pick("പണവും പുരോഗതിയും തമ്മിൽ വ്യത്യാസം", "Payment ahead of progress"),
        FlagKind::CostEscalation => lang.pick("ചെലവ് വർധന", "Cost escalation"),
        FlagKind::Stale => lang.pick("പുതിയ വിവരമില്ല", "No recent update"),
        FlagKind::PaidAboveApproval => lang.pick("അനുവദിച്ചതിലും കൂടുതൽ നൽകി", "Paid above approval"),
    }
}

/// The published rule behind a flag, in one sentence.
pub fn flag_rule(lang: Lang, kind: FlagKind) -> &'static str {
    match kind {
        FlagKind::Overdue => lang.pick(
            "നിശ്ചയിച്ച പൂർത്തീകരണ തീയതി കഴിഞ്ഞിട്ടും പ്രവൃത്തി അവസാനിച്ചതായി (Completed, Foreclosed, Terminated, Package Disposed) കിഫ്ബി രേഖപ്പെടുത്തിയിട്ടില്ലെങ്കിൽ.",
            "Today is past the scheduled completion date and KIIFB does not list the work as ended (Completed, Foreclosed, Terminated or Package Disposed).",
        ),
        FlagKind::PaymentVsProgress => lang.pick(
            "രേഖപ്പെടുത്തിയ സാമ്പത്തിക പുരോഗതി, ഭൗതിക പുരോഗതിയെക്കാൾ 25 ശതമാന പോയിന്റോ അതിലധികമോ കൂടുതലാണെങ്കിൽ.",
            "Reported financial progress is 25 or more percentage points ahead of reported physical progress.",
        ),
        FlagKind::CostEscalation => lang.pick(
            "ഏറ്റവും പുതിയ കണക്കാക്കിയ തുക, ഞങ്ങൾ ആദ്യം രേഖപ്പെടുത്തിയ തുകയുടെ 1.2 മടങ്ങോ അതിലധികമോ ആണെങ്കിൽ.",
            "The latest estimated amount is 1.2 times or more the amount we first recorded.",
        ),
        FlagKind::Stale => lang.pick(
            "പ്രവൃത്തി “Inprogress” ആയിരിക്കെ 90 ദിവസമോ അതിലധികമോ ഡാഷ്ബോർഡിലെ ഒരു വിവരവും മാറിയിട്ടില്ലെങ്കിൽ.",
            "A work is listed as “Inprogress” and no reported figure has changed on the dashboard for 90 days or more.",
        ),
        FlagKind::PaidAboveApproval => lang.pick(
            "കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളിൽ, ഒരു പ്രവൃത്തിക്ക് നൽകിയ തുക അതിന് അനുവദിച്ച തുകയെക്കാൾ 1 ശതമാനത്തിലധികം കൂടുതലാണെങ്കിൽ.",
            "On KIIFB's project status page, the amount paid for a work is more than 1% above the amount approved for it.",
        ),
    }
}

/// The figures behind one raised flag, as a sentence. States what is reported and nothing more.
pub fn flag_explain(lang: Lang, kind: FlagKind, v: &Value) -> String {
    let date = |key: &str| {
        v[key].as_str().and_then(Date::parse_iso).map(Date::to_dmy).unwrap_or_else(|| "?".into())
    };
    let num = |key: &str| v[key].as_f64().map(trim).unwrap_or_else(|| "?".into());
    match kind {
        FlagKind::Overdue => {
            let status = v["status"].as_str().unwrap_or("-");
            match lang {
                Lang::Ml => format!(
                    "നിശ്ചയിച്ച പൂർത്തീകരണ തീയതി {} ആയിരുന്നു. അതിനുശേഷം {} ദിവസം കഴിഞ്ഞു. കിഫ്ബി രേഖപ്പെടുത്തിയ നില: “{status}”.",
                    date("scheduled_end"),
                    num("days_overdue"),
                ),
                Lang::En => format!(
                    "Scheduled completion was {}. That was {} days ago, and KIIFB lists the status as “{status}”.",
                    date("scheduled_end"),
                    num("days_overdue"),
                ),
            }
        }
        FlagKind::PaymentVsProgress => match lang {
            Lang::Ml => format!(
                "രേഖപ്പെടുത്തിയ സാമ്പത്തിക പുരോഗതി {}%, ഭൗതിക പുരോഗതി {}%. വ്യത്യാസം {} പോയിന്റ്.",
                num("financial_pct"),
                num("physical_pct"),
                num("gap_points"),
            ),
            Lang::En => format!(
                "Reported financial progress is {}% and reported physical progress is {}%, a gap of {} points.",
                num("financial_pct"),
                num("physical_pct"),
                num("gap_points"),
            ),
        },
        FlagKind::CostEscalation => {
            let amount = |key: &str| v[key].as_i64().map(crate::fmt::inr).unwrap_or_else(|| "?".into());
            match lang {
                Lang::Ml => format!(
                    "ഞങ്ങൾ ആദ്യം രേഖപ്പെടുത്തിയ കണക്കാക്കിയ തുക {}; ഇപ്പോൾ {}. അനുപാതം {}.",
                    amount("first_amount"),
                    amount("latest_amount"),
                    num("ratio"),
                ),
                Lang::En => format!(
                    "The estimated amount we first recorded was {}; it is now {}, a ratio of {}.",
                    amount("first_amount"),
                    amount("latest_amount"),
                    num("ratio"),
                ),
            }
        }
        FlagKind::Stale => match lang {
            Lang::Ml => format!(
                "ഡാഷ്ബോർഡിലെ വിവരങ്ങൾ അവസാനം മാറിയത് {}-ന്; {} ദിവസമായി മാറ്റമില്ല.",
                date("last_changed"),
                num("days"),
            ),
            Lang::En => format!(
                "The dashboard figures last changed on {}; nothing has changed for {} days.",
                date("last_changed"),
                num("days"),
            ),
        },
        FlagKind::PaidAboveApproval => {
            let amount = |key: &str| v[key].as_i64().map(crate::fmt::inr).unwrap_or_else(|| "?".into());
            match lang {
                Lang::Ml => format!(
                    "ഈ പ്രവൃത്തിക്ക് അനുവദിച്ചത് {}; നൽകിയതായി കിഫ്ബി രേഖപ്പെടുത്തുന്നത് {}. വ്യത്യാസം {}. കാരണം ഉറവിടത്തിൽ പറയുന്നില്ല; പുതുക്കിയ അനുമതി താളിൽ വരാത്തതാകാം.",
                    amount("approved"),
                    amount("paid"),
                    amount("excess"),
                ),
                Lang::En => format!(
                    "{} was approved for this work and KIIFB records {} paid, {} more. The source gives no reason; a revised approval may simply not be shown.",
                    amount("approved"),
                    amount("paid"),
                    amount("excess"),
                ),
            }
        }
    }
}

fn trim(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explains_an_overdue_flag_in_both_languages() {
        let v = json!({"scheduled_end": "2025-12-05", "days_overdue": 299, "status": "Inprogress"});
        let en = flag_explain(Lang::En, FlagKind::Overdue, &v);
        assert_eq!(en, "Scheduled completion was 05-12-2025. That was 299 days ago, and KIIFB lists the status as “Inprogress”.");
        let ml = flag_explain(Lang::Ml, FlagKind::Overdue, &v);
        assert!(ml.contains("05-12-2025") && ml.contains("299") && ml.contains("Inprogress"));
    }

    #[test]
    fn explains_a_mismatch_with_one_decimal() {
        let v = json!({"financial_pct": 60.2, "physical_pct": 30.0, "gap_points": 30.2});
        let en = flag_explain(Lang::En, FlagKind::PaymentVsProgress, &v);
        assert_eq!(en, "Reported financial progress is 60.2% and reported physical progress is 30%, a gap of 30.2 points.");
    }

    #[test]
    fn flag_wording_never_accuses() {
        for lang in [Lang::Ml, Lang::En] {
            for kind in FlagKind::ALL {
                let text = format!("{} {}", flag_label(lang, kind), flag_rule(lang, kind)).to_lowercase();
                for word in ["corrupt", "scam", "fraud", "അഴിമതി", "തട്ടിപ്പ്"] {
                    assert!(!text.contains(word), "{text}");
                }
            }
        }
    }

    #[test]
    fn url_prefixes() {
        assert_eq!(Lang::Ml.prefix(), "");
        assert_eq!(Lang::En.prefix(), "/en");
        assert_eq!(Lang::Ml.other(), Lang::En);
    }
}
