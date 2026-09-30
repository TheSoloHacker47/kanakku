//! Flag rules. Every rule is a plain, published threshold over reported figures.
//! Changing a rule or a threshold means bumping `RULES_VERSION`.

use serde_json::{json, Value};

use crate::model::Project;
use crate::Date;

/// Version 2 (30 Sep 2026): "overdue" no longer fires for works KIIFB lists as foreclosed,
/// terminated or disposed, and "paid above approval" was added.
pub const RULES_VERSION: u32 = 2;

pub const ESCALATION_RATIO: f64 = 1.2;
pub const MISMATCH_POINTS: f64 = 25.0;
pub const STALE_DAYS: i32 = 90;
/// A work is "paid above approval" when payments exceed the approved amount by more than this share.
pub const OVERPAID_PERCENT: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlagKind {
    Overdue,
    PaymentVsProgress,
    CostEscalation,
    Stale,
    /// Raised on works from KIIFB's project status page, not on dashboard projects.
    PaidAboveApproval,
}

impl FlagKind {
    pub const ALL: [FlagKind; 5] =
        [FlagKind::Overdue, FlagKind::PaymentVsProgress, FlagKind::CostEscalation, FlagKind::Stale, FlagKind::PaidAboveApproval];

    /// The rules that are evaluated on dashboard projects.
    pub const DASHBOARD: [FlagKind; 4] =
        [FlagKind::Overdue, FlagKind::PaymentVsProgress, FlagKind::CostEscalation, FlagKind::Stale];

    pub fn as_str(self) -> &'static str {
        match self {
            FlagKind::Overdue => "overdue",
            FlagKind::PaymentVsProgress => "payment_vs_progress",
            FlagKind::CostEscalation => "cost_escalation",
            FlagKind::Stale => "stale",
            FlagKind::PaidAboveApproval => "paid_above_approval",
        }
    }

    pub fn parse(s: &str) -> Option<FlagKind> {
        FlagKind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flag {
    pub kind: FlagKind,
    /// The work the flag is about, or `None` when it concerns the whole project.
    pub work_ref: Option<String>,
    /// The figures the rule was evaluated on.
    pub value: Value,
}

/// What we have observed about a project across earlier snapshots.
#[derive(Clone, Copy, Debug, Default)]
pub struct History {
    /// The estimated amount in the first snapshot we recorded.
    pub first_estimated_amount: Option<i64>,
    /// The last day any reported field of the project changed.
    pub last_changed: Option<Date>,
}

pub fn evaluate(project: &Project, today: Date, history: &History) -> Vec<Flag> {
    let mut flags = Vec::new();

    for (i, work) in project.works.iter().enumerate() {
        if let Some(end) = work.scheduled_end {
            if today > end && !work.is_closed() {
                flags.push(Flag {
                    kind: FlagKind::Overdue,
                    work_ref: Some(work.reference(i)),
                    value: json!({
                        "scheduled_end": end.to_iso(),
                        "days_overdue": today.days_since(end),
                        "status": work.status,
                    }),
                });
            }
        }

        if let (Some(financial), Some(physical)) = (work.financial_pct, work.physical_pct) {
            let gap = financial - physical;
            if gap >= MISMATCH_POINTS {
                flags.push(Flag {
                    kind: FlagKind::PaymentVsProgress,
                    work_ref: Some(work.reference(i)),
                    value: json!({
                        "financial_pct": round1(financial),
                        "physical_pct": round1(physical),
                        "gap_points": round1(gap),
                    }),
                });
            }
        }
    }

    if let (Some(first), Some(latest)) = (history.first_estimated_amount, project.estimated_amount) {
        if first > 0 && latest as f64 / first as f64 >= ESCALATION_RATIO {
            flags.push(Flag {
                kind: FlagKind::CostEscalation,
                work_ref: None,
                value: json!({
                    "first_amount": first,
                    "latest_amount": latest,
                    "ratio": (latest as f64 / first as f64 * 100.0).round() / 100.0,
                }),
            });
        }
    }

    // Only works KIIFB itself lists as in progress are expected to keep changing.
    if let Some(last) = history.last_changed {
        let days = today.days_since(last);
        if days >= STALE_DAYS && project.works.iter().any(|w| w.is_in_progress()) {
            flags.push(Flag {
                kind: FlagKind::Stale,
                work_ref: None,
                value: json!({ "last_changed": last.to_iso(), "days": days }),
            });
        }
    }

    flags
}

/// A flag that is currently open in the database.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenFlag {
    pub id: i64,
    pub kind: FlagKind,
    pub work_ref: Option<String>,
    pub value: Value,
}

/// How to bring the stored open flags of one project in line with a fresh evaluation.
#[derive(Debug, Default, PartialEq)]
pub struct Reconciled {
    /// Newly raised.
    pub raise: Vec<Flag>,
    /// Still raised, with figures that moved (such as days overdue): `(id, new value)`.
    pub update: Vec<(i64, Value)>,
    /// No longer raised: ids to clear.
    pub clear: Vec<i64>,
}

pub fn reconcile(open: &[OpenFlag], computed: Vec<Flag>) -> Reconciled {
    let mut out = Reconciled::default();
    let mut still_open = vec![false; open.len()];
    for flag in computed {
        match open.iter().position(|o| o.kind == flag.kind && o.work_ref == flag.work_ref) {
            Some(i) => {
                still_open[i] = true;
                if open[i].value != flag.value {
                    out.update.push((open[i].id, flag.value));
                }
            }
            None => out.raise.push(flag),
        }
    }
    out.clear = open.iter().zip(still_open).filter(|(_, keep)| !keep).map(|(o, _)| o.id).collect();
    out
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Work;

    fn day(s: &str) -> Date {
        Date::parse_iso(s).unwrap()
    }

    fn project(works: Vec<Work>) -> Project {
        Project { code: "T-1".into(), works, ..Project::default() }
    }

    fn kinds(flags: &[Flag]) -> Vec<FlagKind> {
        flags.iter().map(|f| f.kind).collect()
    }

    #[test]
    fn overdue_needs_a_past_end_date_and_an_unfinished_status() {
        let work = |end: &str, status: &str| Work {
            road_name: Some("Bypass".into()),
            scheduled_end: Some(day(end)),
            status: Some(status.into()),
            ..Work::default()
        };
        let today = day("2026-09-30");
        let none = History::default();

        let flags = evaluate(&project(vec![work("2025-12-05", "Inprogress")]), today, &none);
        assert_eq!(kinds(&flags), [FlagKind::Overdue]);
        assert_eq!(flags[0].work_ref.as_deref(), Some("Bypass"));
        assert_eq!(flags[0].value["days_overdue"], 299);
        assert_eq!(flags[0].value["scheduled_end"], "2025-12-05");

        // Due today is not overdue; one day later is.
        assert!(evaluate(&project(vec![work("2026-09-30", "Inprogress")]), today, &none).is_empty());
        assert_eq!(evaluate(&project(vec![work("2026-09-29", "Inprogress")]), today, &none).len(), 1);
        // Completed works are never overdue, whatever the case of the status text.
        assert!(evaluate(&project(vec![work("2021-01-31", "Completed")]), today, &none).is_empty());
        assert!(evaluate(&project(vec![work("2021-01-31", "COMPLETED")]), today, &none).is_empty());
        // No end date, no flag.
        assert!(evaluate(&project(vec![Work::default()]), today, &none).is_empty());
    }

    #[test]
    fn unnamed_works_are_referenced_by_position() {
        let works = vec![
            Work::default(),
            Work { scheduled_end: Some(day("2025-07-22")), status: Some("Inprogress".into()), ..Work::default() },
        ];
        let flags = evaluate(&project(works), day("2026-09-30"), &History::default());
        assert_eq!(flags[0].work_ref.as_deref(), Some("#2"));
    }

    #[test]
    fn payment_vs_progress_fires_at_25_points() {
        let work = |fin: f64, phys: f64| Work { financial_pct: Some(fin), physical_pct: Some(phys), ..Work::default() };
        let run = |w| evaluate(&project(vec![w]), day("2026-09-30"), &History::default());

        assert!(run(work(60.0, 35.1)).is_empty());
        let flags = run(work(60.0, 35.0));
        assert_eq!(kinds(&flags), [FlagKind::PaymentVsProgress]);
        assert_eq!(flags[0].value["gap_points"], 25.0);
        // Physical progress ahead of payments is not a flag.
        assert!(run(work(36.4, 38.0)).is_empty());
        // A missing figure is a gap, not a mismatch.
        assert!(run(Work { financial_pct: Some(90.0), ..Work::default() }).is_empty());
    }

    #[test]
    fn cost_escalation_compares_against_the_first_recorded_amount() {
        let p = |amount| Project { estimated_amount: Some(amount), ..project(vec![]) };
        let h = |first| History { first_estimated_amount: Some(first), last_changed: None };
        let today = day("2026-09-30");

        assert!(evaluate(&p(119), today, &h(100)).is_empty());
        let flags = evaluate(&p(120), today, &h(100));
        assert_eq!(kinds(&flags), [FlagKind::CostEscalation]);
        assert_eq!(flags[0].value["ratio"], 1.2);
        assert!(evaluate(&p(80), today, &h(100)).is_empty());
        assert!(evaluate(&p(120), today, &History::default()).is_empty());
    }

    #[test]
    fn stale_applies_only_to_projects_with_work_in_progress() {
        let in_progress = Work { status: Some("Inprogress".into()), ..Work::default() };
        let approved = Work { status: Some("Approved".into()), ..Work::default() };
        let h = |d: &str| History { first_estimated_amount: None, last_changed: Some(day(d)) };
        let today = day("2026-09-30");

        assert!(evaluate(&project(vec![in_progress.clone()]), today, &h("2026-07-03")).is_empty()); // 89 days
        let flags = evaluate(&project(vec![in_progress]), today, &h("2026-07-02")); // 90 days
        assert_eq!(kinds(&flags), [FlagKind::Stale]);
        assert_eq!(flags[0].value["days"], 90);
        assert!(evaluate(&project(vec![approved]), today, &h("2020-01-01")).is_empty());
        assert!(evaluate(&project(vec![]), today, &h("2020-01-01")).is_empty());
    }

    #[test]
    fn reconcile_raises_updates_and_clears() {
        let flag = |kind, work: Option<&str>, value: Value| Flag { kind, work_ref: work.map(String::from), value };
        let open = [
            OpenFlag { id: 1, kind: FlagKind::Overdue, work_ref: Some("A".into()), value: json!({"days_overdue": 10}) },
            OpenFlag { id: 2, kind: FlagKind::Overdue, work_ref: Some("B".into()), value: json!({"days_overdue": 5}) },
            OpenFlag { id: 3, kind: FlagKind::Stale, work_ref: None, value: json!({"days": 95}) },
        ];
        let computed = vec![
            flag(FlagKind::Overdue, Some("A"), json!({"days_overdue": 11})),
            flag(FlagKind::Stale, None, json!({"days": 95})),
            flag(FlagKind::PaymentVsProgress, Some("A"), json!({"gap_points": 30.0})),
        ];
        let r = reconcile(&open, computed);
        assert_eq!(r.update, [(1, json!({"days_overdue": 11}))]);
        assert_eq!(r.clear, [2]);
        assert_eq!(r.raise.len(), 1);
        assert_eq!(r.raise[0].kind, FlagKind::PaymentVsProgress);

        assert_eq!(reconcile(&[], vec![]), Reconciled::default());
    }

    #[test]
    fn kind_names_round_trip() {
        for k in FlagKind::ALL {
            assert_eq!(FlagKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(FlagKind::parse("corrupt"), None);
    }
}
