//! The cost guard: this month's Cloudflare usage against what the Workers Paid plan includes.
//!
//! Usage is account-wide, because that is how Cloudflare bills it: other Workers on the same
//! account count too. Figures come from the GraphQL Analytics API and follow the calendar month,
//! which may not match the billing cycle exactly.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    WorkerRequests,
    D1RowsRead,
    D1RowsWritten,
    R2ClassA,
    R2ClassB,
    R2Storage,
}

impl Metric {
    pub const ALL: [Metric; 6] =
        [Metric::WorkerRequests, Metric::D1RowsRead, Metric::D1RowsWritten, Metric::R2ClassA, Metric::R2ClassB, Metric::R2Storage];

    pub fn key(self) -> &'static str {
        match self {
            Metric::WorkerRequests => "worker_requests",
            Metric::D1RowsRead => "d1_rows_read",
            Metric::D1RowsWritten => "d1_rows_written",
            Metric::R2ClassA => "r2_class_a",
            Metric::R2ClassB => "r2_class_b",
            Metric::R2Storage => "r2_storage",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Metric::WorkerRequests => "Worker requests",
            Metric::D1RowsRead => "D1 rows read",
            Metric::D1RowsWritten => "D1 rows written",
            Metric::R2ClassA => "R2 writes and lists (class A)",
            Metric::R2ClassB => "R2 reads (class B)",
            Metric::R2Storage => "R2 storage (bytes)",
        }
    }

    /// What the plan includes each month (Workers Paid; R2's free tier applies on every plan).
    pub fn included(self) -> f64 {
        match self {
            Metric::WorkerRequests => 10e6,
            Metric::D1RowsRead => 25e9,
            Metric::D1RowsWritten => 50e6,
            Metric::R2ClassA => 1e6,
            Metric::R2ClassB => 10e6,
            Metric::R2Storage => 10e9,
        }
    }

    /// US dollars per unit beyond what is included.
    fn price(self) -> f64 {
        match self {
            Metric::WorkerRequests => 0.30 / 1e6,
            Metric::D1RowsRead => 0.001 / 1e6,
            Metric::D1RowsWritten => 1.00 / 1e6,
            Metric::R2ClassA => 4.50 / 1e6,
            Metric::R2ClassB => 0.36 / 1e6,
            Metric::R2Storage => 0.015 / 1e9,
        }
    }

    /// Storage is a level, not a running total, so it is not projected forward.
    fn accumulates(self) -> bool {
        self != Metric::R2Storage
    }
}

/// R2 operations are billed in two classes; deletes and aborted uploads are free.
pub fn r2_class(action: &str) -> Option<Metric> {
    if action.starts_with("Delete") || action.starts_with("Abort") {
        None
    } else if action.starts_with("Get") || action.starts_with("Head") {
        Some(Metric::R2ClassB)
    } else {
        Some(Metric::R2ClassA)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Usage {
    pub worker_requests: f64,
    pub d1_rows_read: f64,
    pub d1_rows_written: f64,
    pub r2_class_a: f64,
    pub r2_class_b: f64,
    pub r2_storage: f64,
}

impl Usage {
    pub fn get(&self, m: Metric) -> f64 {
        match m {
            Metric::WorkerRequests => self.worker_requests,
            Metric::D1RowsRead => self.d1_rows_read,
            Metric::D1RowsWritten => self.d1_rows_written,
            Metric::R2ClassA => self.r2_class_a,
            Metric::R2ClassB => self.r2_class_b,
            Metric::R2Storage => self.r2_storage,
        }
    }

    pub fn add(&mut self, m: Metric, n: f64) {
        match m {
            Metric::WorkerRequests => self.worker_requests += n,
            Metric::D1RowsRead => self.d1_rows_read += n,
            Metric::D1RowsWritten => self.d1_rows_written += n,
            Metric::R2ClassA => self.r2_class_a += n,
            Metric::R2ClassB => self.r2_class_b += n,
            Metric::R2Storage => self.r2_storage = self.r2_storage.max(n),
        }
    }
}

/// One metric this month: used so far, where the month is heading, and the share of the allowance.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Line {
    pub metric: Metric,
    pub used: f64,
    pub projected: f64,
    /// Percent of the allowance used so far.
    pub pct: f64,
    /// Percent of the allowance at month end if the month continues as it has gone so far.
    pub projected_pct: f64,
    /// US dollars beyond the allowance, at the projected month-end figure.
    pub projected_cost: f64,
}

/// `day` is today's day of the month (1-based, counting today as partly used) and `days` the month's length.
pub fn lines(mtd: &Usage, day: u32, days: u32) -> Vec<Line> {
    let elapsed = day.max(1) as f64;
    Metric::ALL
        .iter()
        .map(|&metric| {
            let used = mtd.get(metric);
            let projected = if metric.accumulates() { used / elapsed * days as f64 } else { used };
            Line {
                metric,
                used,
                projected,
                pct: used / metric.included() * 100.0,
                projected_pct: projected / metric.included() * 100.0,
                projected_cost: (projected - metric.included()).max(0.0) * metric.price(),
            }
        })
        .collect()
}

/// Alert levels, as percent of the allowance used so far. `PROJECTED` is the warning that the month
/// is heading past the allowance, raised once there are a few days to project from.
pub const LEVELS: [u32; 3] = [50, 80, 100];
pub const PROJECTED: u32 = 1;
const MIN_DAYS_TO_PROJECT: u32 = 3;

/// Every level a metric has reached. The caller alerts only on those not already alerted this month.
pub fn reached(line: &Line, day: u32) -> Vec<u32> {
    let mut out: Vec<u32> = LEVELS.iter().copied().filter(|&l| line.pct >= l as f64).collect();
    if day >= MIN_DAYS_TO_PROJECT && line.metric.accumulates() && line.projected_pct >= 100.0 && line.pct < 100.0 {
        out.push(PROJECTED);
    }
    out
}

/// A large count, shortly: 1,234 / 12.3 thousand / 4.5 million / 25 billion.
pub fn short(n: f64) -> String {
    let (div, unit) = match n {
        n if n >= 1e9 => (1e9, " billion"),
        n if n >= 1e6 => (1e6, " million"),
        n if n >= 1e4 => (1e3, " thousand"),
        _ => return format!("{}", n.round() as i64),
    };
    let v = n / div;
    let s = if v >= 100.0 { format!("{v:.0}") } else { format!("{v:.1}") };
    format!("{}{unit}", s.trim_end_matches(".0"))
}

fn amount(line: &Line, n: f64) -> String {
    if line.metric == Metric::R2Storage {
        format!("{:.2} GB", n / 1e9)
    } else {
        short(n)
    }
}

/// The Discord message for newly reached levels.
pub fn alert_message(month: &str, hits: &[(Line, u32)]) -> String {
    let mut out = format!("**Kanakku cost guard, {month}** (whole Cloudflare account)\n");
    for (line, level) in hits {
        let m = line.metric;
        let what = if *level == PROJECTED {
            format!("on course to pass its allowance this month: {:.0}% by month end", line.projected_pct)
        } else {
            format!("past {level}% of its allowance")
        };
        out.push_str(&format!(
            "• {} is {what}. {} so far of {} included",
            m.label(),
            amount(line, line.used),
            amount(line, m.included())
        ));
        if m.accumulates() {
            out.push_str(&format!("; heading for {}", amount(line, line.projected)));
        }
        if line.projected_cost >= 0.01 {
            out.push_str(&format!("; about ${:.2} extra", line.projected_cost));
        }
        out.push_str(".\n");
    }
    out.push_str("Check which Worker is responsible in the Cloudflare dashboard, under Workers & Pages and D1.");
    out
}

/// One line per metric for the weekly digest.
pub fn summary(lines: &[Line]) -> String {
    lines
        .iter()
        .map(|l| format!("{} {:.0}%", l.metric.label(), l.pct))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_for(metric: Metric, used: f64, day: u32) -> Line {
        let mut u = Usage::default();
        u.add(metric, used);
        *lines(&u, day, 30).iter().find(|l| l.metric == metric).unwrap()
    }

    #[test]
    fn r2_actions_fall_into_their_billing_class() {
        assert_eq!(r2_class("PutObject"), Some(Metric::R2ClassA));
        assert_eq!(r2_class("ListObjects"), Some(Metric::R2ClassA));
        assert_eq!(r2_class("GetObject"), Some(Metric::R2ClassB));
        assert_eq!(r2_class("HeadBucket"), Some(Metric::R2ClassB));
        assert_eq!(r2_class("DeleteObject"), None);
    }

    #[test]
    fn projection_scales_running_totals_but_not_storage() {
        let reads = line_for(Metric::D1RowsRead, 5e9, 10);
        assert_eq!(reads.projected, 15e9);
        assert_eq!(reads.pct, 20.0);
        assert_eq!(reads.projected_cost, 0.0);
        let storage = line_for(Metric::R2Storage, 2e9, 10);
        assert_eq!(storage.projected, 2e9);
    }

    #[test]
    fn levels_are_reached_in_order_and_projection_needs_a_few_days() {
        assert_eq!(reached(&line_for(Metric::D1RowsRead, 13e9, 20), 20), vec![50]);
        assert_eq!(reached(&line_for(Metric::D1RowsRead, 21e9, 25), 25), vec![50, 80, PROJECTED]);
        assert_eq!(reached(&line_for(Metric::D1RowsRead, 26e9, 28), 28), vec![50, 80, 100]);
        // One heavy first day projects past the allowance, but that is too little to go on.
        assert_eq!(reached(&line_for(Metric::D1RowsRead, 2e9, 1), 1), Vec::<u32>::new());
        assert_eq!(reached(&line_for(Metric::D1RowsRead, 3e9, 3), 3), vec![PROJECTED]);
    }

    #[test]
    fn overage_is_priced() {
        let requests = line_for(Metric::WorkerRequests, 20e6, 30);
        assert!((requests.projected_cost - 3.0).abs() < 1e-9);
    }

    #[test]
    fn messages_read_plainly() {
        assert_eq!(short(517_420_265.0), "517 million");
        assert_eq!(short(25e9), "25 billion");
        assert_eq!(short(12_345.0), "12.3 thousand");
        assert_eq!(short(950.0), "950");
        let msg = alert_message("October 2026", &[(line_for(Metric::D1RowsRead, 13e9, 10), 50)]);
        assert!(msg.contains("D1 rows read is past 50% of its allowance. 13 billion so far of 25 billion included; heading for 39 billion; about $14.00 extra."), "{msg}");
    }
}
