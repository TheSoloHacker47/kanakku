//! KIIFB's status strings are workflow jargon ("WBS Base One Approved"). We group them into a
//! few plain stages. The grouping is ours; the original status is always shown beside it.

use crate::i18n::Lang;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Preparation,
    TechnicalSanction,
    Tender,
    Contract,
    WorkPlan,
    /// Sent back to the implementing agency. Not a step on the track.
    Returned,
}

impl Stage {
    /// The stages a project moves through, in order.
    pub const TRACK: [Stage; 5] =
        [Stage::Preparation, Stage::TechnicalSanction, Stage::Tender, Stage::Contract, Stage::WorkPlan];

    pub const ALL: [Stage; 6] =
        [Stage::Preparation, Stage::TechnicalSanction, Stage::Tender, Stage::Contract, Stage::WorkPlan, Stage::Returned];

    pub fn from_status(status: &str) -> Option<Stage> {
        let s = status.to_lowercase();
        let has = |needle: &str| s.contains(needle);
        Some(if has("return") {
            Stage::Returned
        } else if has("wbs") || has("payment milestone") {
            Stage::WorkPlan
        } else if has("contract") {
            Stage::Contract
        } else if has("tender") {
            Stage::Tender
        } else if has("technical sanction") || has("project execution document") {
            // KIIFB's guidelines put the Project Execution Document after technical sanction.
            Stage::TechnicalSanction
        } else if has("project") || has("design basis") || has("submitted") {
            Stage::Preparation
        } else {
            return None;
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Preparation => "preparation",
            Stage::TechnicalSanction => "technical_sanction",
            Stage::Tender => "tender",
            Stage::Contract => "contract",
            Stage::WorkPlan => "work_plan",
            Stage::Returned => "returned",
        }
    }

    pub fn parse(s: &str) -> Option<Stage> {
        Stage::ALL.into_iter().find(|stage| stage.as_str() == s)
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Stage::Preparation => lang.pick("തയ്യാറെടുപ്പ്", "Preparation"),
            Stage::TechnicalSanction => lang.pick("സാങ്കേതികാനുമതി", "Technical sanction"),
            Stage::Tender => lang.pick("ടെൻഡർ", "Tender"),
            Stage::Contract => lang.pick("കരാർ", "Contract"),
            Stage::WorkPlan => lang.pick("പ്രവൃത്തി പദ്ധതി", "Work plan"),
            Stage::Returned => lang.pick("ഏജൻസിക്ക് തിരിച്ചയച്ചു", "Returned to agency"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_every_status_seen_on_the_dashboard() {
        let cases = [
            ("WBS Base One Approved", Stage::WorkPlan),
            ("WBS Base Zero Approved", Stage::WorkPlan),
            ("WBS Approval Created", Stage::WorkPlan),
            ("Payment milestone finalization pending base 1", Stage::WorkPlan),
            ("Technical Sanction Created", Stage::TechnicalSanction),
            ("Technical Sanction Completed", Stage::TechnicalSanction),
            ("Contract Process Completed", Stage::Contract),
            ("Contract Process Created", Stage::Contract),
            ("Tender Completed", Stage::Tender),
            ("Tender Created", Stage::Tender),
            ("Project Created", Stage::Preparation),
            ("Project Initiated", Stage::Preparation),
            ("Project Execution Document Stage 1 Approved", Stage::TechnicalSanction),
            ("Project Execution Document Stage 2 Initiated", Stage::TechnicalSanction),
            ("Design Basis Report Approved", Stage::Preparation),
            ("Submitted for Confirmation by KIIFB", Stage::Preparation),
            ("Return to SPV", Stage::Returned),
        ];
        for (status, stage) in cases {
            assert_eq!(Stage::from_status(status), Some(stage), "{status}");
        }
        assert_eq!(Stage::from_status("Something new"), None);
    }

    #[test]
    fn names_round_trip_and_the_track_excludes_returned() {
        for stage in Stage::ALL {
            assert_eq!(Stage::parse(stage.as_str()), Some(stage));
        }
        assert!(!Stage::TRACK.contains(&Stage::Returned));
    }
}
