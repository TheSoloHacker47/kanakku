//! What changed in a project's record between two snapshots.

use crate::model::{Project, Work};

#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub field: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

pub fn diff(old: &Project, new: &Project) -> Vec<Change> {
    let mut out = Vec::new();
    let mut push = |field: String, a: Option<String>, b: Option<String>| {
        if a != b {
            out.push(Change { field, old: a, new: b });
        }
    };

    push("title".into(), Some(old.title.clone()), Some(new.title.clone()));
    push("department".into(), old.department.clone(), new.department.clone());
    push("executing_agency".into(), old.executing_agency.clone(), new.executing_agency.clone());
    push("status".into(), old.status.clone(), new.status.clone());
    push("estimated_amount".into(), text(old.estimated_amount), text(new.estimated_amount));
    push("expenditure".into(), text(old.expenditure), text(new.expenditure));

    let refs = |p: &Project| p.works.iter().enumerate().map(|(i, w)| w.reference(i)).collect::<Vec<_>>();
    let (old_refs, new_refs) = (refs(old), refs(new));

    for (i, name) in new_refs.iter().enumerate() {
        let after = &new.works[i];
        let Some(j) = old_refs.iter().position(|r| r == name) else {
            push(format!("work[{name}]"), None, Some("listed".into()));
            continue;
        };
        for (field, a, b) in work_fields(&old.works[j], after) {
            push(format!("work[{name}].{field}"), a, b);
        }
    }
    for name in old_refs.iter().filter(|r| !new_refs.contains(r)) {
        push(format!("work[{name}]"), Some("listed".into()), None);
    }

    out
}

fn work_fields(a: &Work, b: &Work) -> [(&'static str, Option<String>, Option<String>); 9] {
    [
        ("status", a.status.clone(), b.status.clone()),
        ("contractor", a.contractor.clone(), b.contractor.clone()),
        ("contract_amount", text(a.contract_amount), text(b.contract_amount)),
        ("paid_amount", text(a.paid_amount), text(b.paid_amount)),
        ("scheduled_start", a.scheduled_start.map(|d| d.to_iso()), b.scheduled_start.map(|d| d.to_iso())),
        ("scheduled_end", a.scheduled_end.map(|d| d.to_iso()), b.scheduled_end.map(|d| d.to_iso())),
        ("physical_pct", text(a.physical_pct), text(b.physical_pct)),
        ("financial_pct", text(a.financial_pct), text(b.financial_pct)),
        ("progress_note", a.progress_note.clone(), b.progress_note.clone()),
    ]
}

fn text<T: ToString>(v: Option<T>) -> Option<String> {
    v.map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Date;

    fn base() -> Project {
        Project {
            code: "PWD007-01-02".into(),
            title: "Bypass".into(),
            estimated_amount: Some(100),
            works: vec![
                Work { road_name: Some("North".into()), physical_pct: Some(25.0), ..Work::default() },
                Work { road_name: Some("South".into()), ..Work::default() },
            ],
            ..Project::default()
        }
    }

    #[test]
    fn identical_records_have_no_changes() {
        assert!(diff(&base(), &base()).is_empty());
    }

    #[test]
    fn reports_project_and_work_fields() {
        let mut new = base();
        new.estimated_amount = Some(130);
        new.status = Some("Tender Completed".into());
        new.works[0].physical_pct = Some(40.0);
        new.works[0].scheduled_end = Date::parse_iso("2027-03-31");

        let changes = diff(&base(), &new);
        let as_tuples: Vec<_> =
            changes.iter().map(|c| (c.field.as_str(), c.old.as_deref(), c.new.as_deref())).collect();
        assert_eq!(
            as_tuples,
            [
                ("status", None, Some("Tender Completed")),
                ("estimated_amount", Some("100"), Some("130")),
                ("work[North].scheduled_end", None, Some("2027-03-31")),
                ("work[North].physical_pct", Some("25"), Some("40")),
            ]
        );
    }

    #[test]
    fn reports_works_appearing_and_disappearing() {
        let mut new = base();
        new.works.remove(1);
        new.works.push(Work { road_name: Some("East".into()), ..Work::default() });
        let fields: Vec<_> = diff(&base(), &new).into_iter().map(|c| (c.field, c.old, c.new)).collect();
        assert_eq!(
            fields,
            [
                ("work[East]".to_string(), None, Some("listed".to_string())),
                ("work[South]".to_string(), Some("listed".to_string()), None),
            ]
        );
    }

    #[test]
    fn reordering_works_is_not_a_change() {
        let mut new = base();
        new.works.swap(0, 1);
        assert!(diff(&base(), &new).is_empty());
    }
}
