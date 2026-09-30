//! Open data: the same records the pages show, as JSON, CSV and GeoJSON.

use kanakku_core::kiifb;
use kanakku_core::model::Project;
use serde_json::{json, Value};

use crate::db::{ExportRow, ProjectPage};

fn flag_list(types: Option<&str>) -> Vec<&str> {
    types.map(|t| t.split(',').collect()).unwrap_or_default()
}

pub fn projects_json(rows: &[ExportRow], last_checked: Option<&str>) -> String {
    let projects: Vec<Value> = rows
        .iter()
        .filter_map(|row| {
            let mut record: Value = serde_json::from_str(&row.record_json).ok()?;
            let object = record.as_object_mut()?;
            object.insert("open_flags".into(), json!(flag_list(row.flag_types.as_deref())));
            object.insert("first_seen_on".into(), json!(row.first_seen_on));
            object.insert("changed_on".into(), json!(row.changed_on));
            object.insert("missing_since".into(), json!(row.missing_since));
            Some(record)
        })
        .collect();
    json!({
        "source": kiifb::SOURCE_URL,
        "source_last_checked": last_checked,
        "currency": "INR, whole rupees",
        "count": projects.len(),
        "projects": projects,
    })
    .to_string()
}

pub fn project_json(project: &Project, data: &ProjectPage) -> String {
    let flags: Vec<Value> = data
        .flags
        .iter()
        .map(|f| {
            json!({
                "type": f.kind,
                "work": f.work_ref,
                "rule_version": f.rule_version,
                "value": serde_json::from_str::<Value>(&f.value_json).unwrap_or_default(),
                "status": f.status,
                "raised_on": f.created_on,
                "cleared_on": f.cleared_on,
            })
        })
        .collect();
    let changes: Vec<Value> = data
        .observations
        .iter()
        .map(|o| json!({ "field": o.field, "old": o.old_value, "new": o.new_value, "observed_on": o.observed_on }))
        .collect();
    json!({
        "project": project,
        "flags": flags,
        "changes": changes,
        "first_seen_on": data.row.first_seen_on,
        "missing_since": data.row.missing_since,
        "source": {
            "url": kiifb::SOURCE_URL,
            "retrieved_at": data.row.fetched_at,
            "sha256": data.row.sha256,
            "snapshot": format!("/snapshot/{}", data.row.snapshot_id),
        },
    })
    .to_string()
}

pub fn projects_csv(rows: &[ExportRow]) -> String {
    let mut out = String::from(
        "code,title,department,sector,executing_agency,district,constituencies,estimated_amount,expenditure,works_amount,status,works,contractors,open_flags,latitude,longitude,first_seen_on,changed_on,missing_since\r\n",
    );
    for row in rows {
        let Ok(p) = serde_json::from_str::<Project>(&row.record_json) else { continue };
        let num = |v: Option<i64>| v.map(|v| v.to_string()).unwrap_or_default();
        let join = |items: Vec<&str>| items.join("; ");
        let (lat, lng) = p
            .sites
            .first()
            .map(|s| (s.lat, s.lng))
            .or_else(|| p.works.iter().find_map(|w| Some((w.lat?, w.lng?))))
            .map(|(lat, lng)| (lat.to_string(), lng.to_string()))
            .unwrap_or_default();
        let cells = [
            p.code.clone(),
            p.title.clone(),
            p.department.clone().unwrap_or_default(),
            p.sector.clone().unwrap_or_default(),
            p.executing_agency.clone().unwrap_or_default(),
            p.district.clone(),
            join(p.constituencies.iter().map(|c| c.name.as_str()).collect()),
            num(p.estimated_amount),
            num(p.expenditure),
            num(p.works_amount()),
            p.status.clone().unwrap_or_default(),
            p.works.len().to_string(),
            join(p.works.iter().filter_map(|w| w.contractor.as_deref()).collect()),
            join(flag_list(row.flag_types.as_deref())),
            lat,
            lng,
            row.first_seen_on.clone(),
            row.changed_on.clone(),
            row.missing_since.clone().unwrap_or_default(),
        ];
        for (i, cell) in cells.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_cell(&mut out, cell);
        }
        out.push_str("\r\n");
    }
    out
}

/// Quotes a CSV cell, and defuses values a spreadsheet would run as a formula.
fn push_cell(out: &mut String, cell: &str) {
    let risky = cell.starts_with(['=', '+', '-', '@', '\t', '\r']) && cell.parse::<f64>().is_err();
    if !risky && !cell.contains([',', '"', '\n', '\r']) {
        out.push_str(cell);
        return;
    }
    out.push('"');
    if risky {
        out.push('\'');
    }
    out.push_str(&cell.replace('"', "\"\""));
    out.push('"');
}

pub fn projects_geojson(rows: &[ExportRow]) -> String {
    let mut features = Vec::new();
    for row in rows {
        let Ok(p) = serde_json::from_str::<Project>(&row.record_json) else { continue };
        let flags = flag_list(row.flag_types.as_deref());
        let amount = p.estimated_amount.or(p.works_amount());
        let mut point = |lat: f64, lng: f64, work: Option<String>| {
            features.push(json!({
                "type": "Feature",
                "geometry": { "type": "Point", "coordinates": [round6(lng), round6(lat)] },
                "properties": { "code": p.code, "title": p.title, "work": work, "amount": amount, "flagged": !flags.is_empty(), "flags": flags },
            }));
        };
        for site in &p.sites {
            point(site.lat, site.lng, None);
        }
        for (i, w) in p.works.iter().enumerate() {
            if let (Some(lat), Some(lng)) = (w.lat, w.lng) {
                point(lat, lng, Some(w.reference(i)));
            }
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

fn round6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}
