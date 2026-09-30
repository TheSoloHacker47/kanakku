use serde::{Deserialize, Serialize};

use crate::Date;

/// One KIIFB project, identified by its project code. All money is in whole rupees.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub code: String,
    pub title: String,
    pub department: Option<String>,
    pub sector: Option<String>,
    pub executing_agency: Option<String>,
    /// The first district KIIFB files the project under; empty when it states none.
    pub district: String,
    /// Every district it is filed under. A few projects have pins in more than one.
    #[serde(default)]
    pub districts: Vec<String>,
    /// KIIFB lists some projects under more than one assembly constituency.
    pub constituencies: Vec<Constituency>,
    /// KIIFB states the estimate for the parent sub-project, so sibling packages repeat it.
    pub estimated_amount: Option<i64>,
    /// The parent sub-project the estimate belongs to.
    #[serde(default)]
    pub sub_project_code: Option<String>,
    /// How many contract packages across the state carry this same estimate. 1 means it is this project's own.
    #[serde(default)]
    pub estimate_shared_by: u32,
    /// Expenditure summed over all packages that share the estimate.
    #[serde(default)]
    pub group_expenditure: Option<i64>,
    pub expenditure: Option<i64>,
    pub status: Option<String>,
    pub sites: Vec<Site>,
    pub works: Vec<Work>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Constituency {
    pub name: String,
    pub name_ml: Option<String>,
    pub mla_name: Option<String>,
    pub mla_name_ml: Option<String>,
}

/// A pin on the map. A project can have several.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub lat: f64,
    pub lng: f64,
}

/// A road or bridge work under a project, from KIIFB's transport layer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Work {
    pub road_name: Option<String>,
    pub spv: Option<String>,
    pub contractor: Option<String>,
    pub as_amount: Option<i64>,
    pub fs_amount: Option<i64>,
    pub ts_amount: Option<i64>,
    pub tender_amount: Option<i64>,
    pub loa_amount: Option<i64>,
    pub contract_amount: Option<i64>,
    pub paid_amount: Option<i64>,
    pub paid_contractor: Option<i64>,
    pub scheduled_start: Option<Date>,
    pub scheduled_end: Option<Date>,
    pub progress_note: Option<String>,
    pub physical_pct: Option<f64>,
    pub financial_pct: Option<f64>,
    pub status: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

/// What the one prominent figure for a project stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Headline {
    /// The project's own estimate.
    Estimate,
    /// Spending on this package. Used when the estimate belongs to a whole sub-project.
    Spent,
    /// The sum of the works' financial sanctions.
    WorksTotal,
}

/// Picks the figure that describes this project alone. A sub-project estimate shared by
/// many packages is never the headline: it would credit one package with the whole programme.
pub fn headline(
    estimated_amount: Option<i64>,
    estimate_shared_by: u32,
    expenditure: Option<i64>,
    works_amount: Option<i64>,
) -> Option<(i64, Headline)> {
    match estimated_amount {
        Some(estimate) if estimate_shared_by <= 1 => Some((estimate, Headline::Estimate)),
        Some(_) => expenditure.map(|spent| (spent, Headline::Spent)),
        None => works_amount.map(|total| (total, Headline::WorksTotal)),
    }
}

impl Project {
    pub fn headline(&self) -> Option<(i64, Headline)> {
        headline(self.estimated_amount, self.estimate_shared_by, self.expenditure, self.works_amount())
    }

    /// True when the estimate belongs to a sub-project that several packages share.
    pub fn estimate_is_shared(&self) -> bool {
        self.estimate_shared_by > 1
    }

    /// Sum of the works' financial sanction amounts, for projects KIIFB lists only as works.
    pub fn works_amount(&self) -> Option<i64> {
        let total: i64 = self.works.iter().filter_map(|w| w.fs_amount).sum();
        (total > 0).then_some(total)
    }
}

impl Work {
    /// How a work is referred to in flags and on the page: its road name, or its position.
    pub fn reference(&self, index: usize) -> String {
        match &self.road_name {
            Some(name) => name.clone(),
            None => format!("#{}", index + 1),
        }
    }

    /// KIIFB draws one contract as several line segments; they repeat the same figures.
    pub fn is_segment_of(&self, other: &Work) -> bool {
        self.fs_amount == other.fs_amount
            && self.contract_amount == other.contract_amount
            && self.paid_amount == other.paid_amount
            && self.contractor == other.contractor
            && self.scheduled_start == other.scheduled_start
            && self.scheduled_end == other.scheduled_end
            && self.status == other.status
            && (self.road_name.is_none() || other.road_name.is_none() || self.road_name == other.road_name)
    }

    pub fn is_completed(&self) -> bool {
        self.status.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("completed"))
    }

    /// The contract has ended, whether by completion or otherwise, so its schedule no longer runs.
    pub fn is_closed(&self) -> bool {
        self.status.as_deref().is_some_and(|s| {
            ["completed", "foreclosed", "terminated", "package disposed"].iter().any(|closed| s.eq_ignore_ascii_case(closed))
        })
    }

    pub fn is_in_progress(&self) -> bool {
        self.status.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("inprogress"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_estimate_is_never_the_headline() {
        assert_eq!(headline(Some(500), 1, Some(20), None), Some((500, Headline::Estimate)));
        assert_eq!(headline(Some(500), 148, Some(20), None), Some((20, Headline::Spent)));
        assert_eq!(headline(Some(500), 148, None, None), None);
        assert_eq!(headline(None, 0, None, Some(90)), Some((90, Headline::WorksTotal)));
        assert_eq!(headline(None, 0, None, None), None);
    }
}
