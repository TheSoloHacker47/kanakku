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
    pub district: String,
    /// KIIFB lists some projects under more than one assembly constituency.
    pub constituencies: Vec<Constituency>,
    pub estimated_amount: Option<i64>,
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

impl Project {
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

    pub fn is_completed(&self) -> bool {
        self.status.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("completed"))
    }

    pub fn is_in_progress(&self) -> bool {
        self.status.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("inprogress"))
    }
}
