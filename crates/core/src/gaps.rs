//! "What we don't know yet": fields the source does not publish for a project.

use crate::i18n::Lang;
use crate::model::Project;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapKind {
    /// KIIFB lists no works for the project, so contract, schedule and payment details are all absent.
    WorkDetails,
    EstimatedAmount,
    Expenditure,
    Location,
    Contractor,
    Schedule,
    Payments,
    PhysicalProgress,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gap {
    pub kind: GapKind,
    /// The work the gap concerns, or `None` for the project as a whole.
    pub work_ref: Option<String>,
}

pub fn find(project: &Project) -> Vec<Gap> {
    let mut gaps = Vec::new();
    let mut project_gap = |kind| gaps.push(Gap { kind, work_ref: None });

    if project.works.is_empty() {
        project_gap(GapKind::WorkDetails);
        if project.estimated_amount.is_none() {
            project_gap(GapKind::EstimatedAmount);
        }
        if project.expenditure.is_none() {
            project_gap(GapKind::Expenditure);
        }
    }
    if project.sites.is_empty() && project.works.iter().all(|w| w.lat.is_none()) {
        project_gap(GapKind::Location);
    }

    for (i, work) in project.works.iter().enumerate() {
        let mut work_gap = |kind| gaps.push(Gap { kind, work_ref: Some(work.reference(i)) });
        if work.contractor.is_none() {
            work_gap(GapKind::Contractor);
        }
        if work.scheduled_start.is_none() || work.scheduled_end.is_none() {
            work_gap(GapKind::Schedule);
        }
        if work.paid_amount.is_none() {
            work_gap(GapKind::Payments);
        }
        if work.physical_pct.is_none() {
            work_gap(GapKind::PhysicalProgress);
        }
    }
    gaps
}

pub fn label(lang: Lang, kind: GapKind) -> &'static str {
    match kind {
        GapKind::WorkDetails => lang.pick(
            "കരാറുകാരൻ, കരാർ തുക, നിശ്ചയിച്ച തീയതികൾ, നൽകിയ തുക, ഭൗതിക പുരോഗതി",
            "Contractor, contract amount, scheduled dates, payments and physical progress",
        ),
        GapKind::EstimatedAmount => lang.pick("കണക്കാക്കിയ തുക", "Estimated amount"),
        GapKind::Expenditure => lang.pick("ഇതുവരെയുള്ള ചെലവ്", "Expenditure so far"),
        GapKind::Location => lang.pick("പദ്ധതിയുടെ സ്ഥാനം", "Project location"),
        GapKind::Contractor => lang.pick("കരാറുകാരൻ", "Contractor"),
        GapKind::Schedule => lang.pick("നിശ്ചയിച്ച തുടക്ക, പൂർത്തീകരണ തീയതികൾ", "Scheduled start and completion dates"),
        GapKind::Payments => lang.pick("നൽകിയ തുക", "Amount paid"),
        GapKind::PhysicalProgress => lang.pick("ഭൗതിക പുരോഗതി", "Physical progress"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Site, Work};
    use crate::Date;

    fn kinds(p: &Project) -> Vec<(GapKind, Option<String>)> {
        find(p).into_iter().map(|g| (g.kind, g.work_ref)).collect()
    }

    #[test]
    fn a_pin_only_project_lacks_all_work_details() {
        let p = Project { estimated_amount: Some(1), sites: vec![Site { lat: 10.0, lng: 76.3 }], ..Project::default() };
        assert_eq!(kinds(&p), [(GapKind::WorkDetails, None), (GapKind::Expenditure, None)]);
    }

    #[test]
    fn work_gaps_name_the_work() {
        let complete = Work {
            road_name: Some("Flyover".into()),
            contractor: Some("X".into()),
            scheduled_start: Date::parse_iso("2017-11-17"),
            scheduled_end: Date::parse_iso("2026-01-31"),
            paid_amount: Some(1),
            physical_pct: Some(100.0),
            lat: Some(9.97),
            lng: Some(76.32),
            ..Work::default()
        };
        let bare = Work { lat: Some(9.9), lng: Some(76.3), scheduled_start: Date::parse_iso("2024-01-01"), ..Work::default() };
        let p = Project { works: vec![complete, bare], ..Project::default() };
        let second = Some("#2".to_string());
        assert_eq!(
            kinds(&p),
            [
                (GapKind::Contractor, second.clone()),
                (GapKind::Schedule, second.clone()),
                (GapKind::Payments, second.clone()),
                (GapKind::PhysicalProgress, second),
            ]
        );
    }

    #[test]
    fn location_is_a_gap_only_when_nothing_has_coordinates() {
        let p = Project { works: vec![Work::default()], ..Project::default() };
        assert!(kinds(&p).contains(&(GapKind::Location, None)));
    }
}
