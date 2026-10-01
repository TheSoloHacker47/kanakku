//! Open data: the same records the pages show, as JSON, CSV and GeoJSON.

use kanakku_core::kiifb;
use kanakku_core::kiifb_status::{self, FundedProject};
use kanakku_core::model::Project;
use serde_json::{json, Value};

use crate::db::{ExportRow, FundingExportRow, LiabilityExportRow, ProjectPage, SourceStatus, Status};
use crate::runs::STALE_AFTER_HOURS;

/// The terms every download carries. The figures are the sources'; what Kanakku adds is CC BY 4.0.
pub const LICENSE: &str = "CC-BY-4.0";
pub const LICENSE_URL: &str = "https://creativecommons.org/licenses/by/4.0/";
pub const ATTRIBUTION: &str = "Figures as published by the source named in each record. Compiled, cleaned, flagged and joined by Kanakku. Credit both.";

/// A JSON body with the licence and attribution added at the top level.
fn with_terms(mut body: Value) -> String {
    if let Some(object) = body.as_object_mut() {
        object.insert("license".into(), json!(LICENSE));
        object.insert("license_url".into(), json!(LICENSE_URL));
        object.insert("attribution".into(), json!(ATTRIBUTION));
    }
    body.to_string()
}

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
    with_terms(json!({
        "source": kiifb::SOURCE_URL,
        "source_last_checked": last_checked,
        "currency": "INR, whole rupees",
        "count": projects.len(),
        "projects": projects,
    }))
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
    with_terms(json!({
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
    }))
}

pub fn projects_csv(rows: &[ExportRow]) -> String {
    let mut out = String::from(
        "code,title,department,sector,executing_agency,district,constituencies,estimated_amount,sub_project_code,estimate_shared_by,sub_project_expenditure,expenditure,works_amount,status,works,contractors,open_flags,latitude,longitude,first_seen_on,changed_on,missing_since\r\n",
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
            p.sub_project_code.clone().unwrap_or_default(),
            p.estimate_shared_by.to_string(),
            num(p.group_expenditure),
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
        let amount = p.headline().map(|(amount, _)| amount);
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
    with_terms(json!({ "type": "FeatureCollection", "features": features }))
}

fn round6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

pub fn funding_json(rows: &[FundingExportRow], last_checked: Option<&str>) -> String {
    let projects: Vec<Value> = rows
        .iter()
        .filter_map(|row| {
            let project: FundedProject = serde_json::from_str(&row.record_json).ok()?;
            let mut record: Value = serde_json::from_str(&row.record_json).ok()?;
            let object = record.as_object_mut()?;
            // KIIFB's list over-counts payments for projects filed under several districts.
            object.insert("released_as_listed".into(), json!(project.released));
            object.remove("released");
            object.insert("paid".into(), json!(project.paid()));
            object.insert("listed_multiple".into(), json!(project.listed_multiple()));
            object.insert("map_group".into(), json!(row.group_key));
            object.insert("map_group_basis".into(), json!(row.match_basis));
            object.insert("first_seen_on".into(), json!(row.first_seen_on));
            object.insert("changed_on".into(), json!(row.changed_on));
            object.insert("missing_since".into(), json!(row.missing_since));
            Some(record)
        })
        .collect();
    with_terms(json!({
        "source": kiifb_status::SOURCE_URL,
        "source_last_checked": last_checked,
        "currency": "INR, whole rupees",
        "count": projects.len(),
        "projects": projects,
    }))
}

/// One row per work; a project without works gets one row with the work columns empty.
pub fn funding_csv(rows: &[FundingExportRow]) -> String {
    let mut out = String::from(
        "project_ref,project,department,spv,announced_under,project_status,project_approved,project_paid,project_released_as_listed,map_group,work_no,work,work_spv,work_status,work_approved,work_paid,changed_on,missing_since\r\n",
    );
    let num = |v: Option<i64>| v.map(|v| v.to_string()).unwrap_or_default();
    for row in rows {
        let Ok(p) = serde_json::from_str::<FundedProject>(&row.record_json) else { continue };
        let head = [
            p.reference.clone(),
            p.name.clone(),
            p.department.clone().unwrap_or_default(),
            p.spv.clone().unwrap_or_default(),
            p.main_project.clone().unwrap_or_default(),
            p.status.clone().unwrap_or_default(),
            num(p.approved),
            num(p.paid()),
            num(p.released),
            row.group_key.clone().unwrap_or_default(),
        ];
        let tail = [row.changed_on.clone(), row.missing_since.clone().unwrap_or_default()];
        let mut line = |work: [String; 6]| {
            for (i, cell) in head.iter().chain(work.iter()).chain(tail.iter()).enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_cell(&mut out, cell);
            }
            out.push_str("\r\n");
        };
        if p.works.is_empty() {
            line(Default::default());
        }
        for (i, w) in p.works.iter().enumerate() {
            line([
                (i + 1).to_string(),
                w.name.clone(),
                w.spv.clone().unwrap_or_default(),
                w.status.clone().unwrap_or_default(),
                num(w.approved),
                num(w.paid),
            ]);
        }
    }
    out
}

pub fn liability_json(rows: &[LiabilityExportRow]) -> String {
    let works: Vec<Value> = rows
        .iter()
        .map(|w| {
            json!({
                "office_district": w.district,
                "wing": w.wing,
                "work": w.name,
                "contractor": w.contractor,
                "agreed_amount": w.agreed_amount,
                "liability_starts": w.starts_on,
                "liability_ends": w.ends_on,
                "division": w.division,
                "subdivision": w.subdivision,
                "first_seen_on": w.first_seen_on,
                "missing_since": w.missing_since,
            })
        })
        .collect();
    with_terms(json!({ "source": kanakku_core::pwd_dlp::SOURCE_URL, "currency": "INR, whole rupees", "count": works.len(), "works": works }))
}

pub fn liability_csv(rows: &[LiabilityExportRow]) -> String {
    let mut out = String::from("office_district,wing,work,contractor,agreed_amount,liability_starts,liability_ends,division,subdivision,first_seen_on,missing_since\r\n");
    for w in rows {
        let amount = w.agreed_amount.map(|a| a.to_string()).unwrap_or_default();
        let cells = [
            w.district.as_deref().unwrap_or(""),
            w.wing.as_str(),
            w.name.as_str(),
            w.contractor.as_deref().unwrap_or(""),
            amount.as_str(),
            w.starts_on.as_deref().unwrap_or(""),
            w.ends_on.as_deref().unwrap_or(""),
            w.division.as_deref().unwrap_or(""),
            w.subdivision.as_deref().unwrap_or(""),
            w.first_seen_on.as_str(),
            w.missing_since.as_deref().unwrap_or(""),
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

/// Hours since a source was last read successfully, and whether that is too long.
pub fn freshness(source: &SourceStatus, now_ms: i64) -> (Option<i64>, bool) {
    let last = source.last_ok_at.as_deref().or(source.last_scraped_at.as_deref());
    let age = last.map(|at| (now_ms - worker::js_sys::Date::new(&at.into()).get_time() as i64) / 3_600_000);
    let limit = STALE_AFTER_HOURS.iter().find(|(id, _)| *id == source.id).map(|(_, hours)| *hours).unwrap_or(36);
    (age, age.is_none_or(|hours| hours > limit))
}

/// The health check: every source, how old its last good read is, and whether any is stale.
/// Returns the body and whether everything is fresh.
pub fn status_json(status: &Status, now_ms: i64) -> (String, bool) {
    let mut healthy = true;
    let sources: Vec<Value> = status
        .sources
        .iter()
        .map(|source| {
            let (age, stale) = freshness(source, now_ms);
            healthy &= !stale;
            json!({
                "source": source.name,
                "url": source.base_url,
                "last_good_read": source.last_ok_at.as_deref().or(source.last_scraped_at.as_deref()),
                "hours_since": age,
                "stale": stale,
                "last_run_ok": source.last_run_ok.map(|ok| ok == 1),
            })
        })
        .collect();
    (json!({ "healthy": healthy, "sources": sources }).to_string(), healthy)
}

/// A page of a paged endpoint: the items, where this page sits, and the address of the next one.
fn paged(key: &str, items: Vec<Value>, page: u32, total: u32, next: Option<String>) -> String {
    let mut body = json!({ "page": page, "per_page": crate::db::API_PAGE_SIZE, "total": total, "next": next });
    body[key] = Value::Array(items);
    with_terms(body)
}

pub fn changes_json(rows: &[crate::db::ChangeRow], page: u32, total: u32, next: Option<String>) -> String {
    let items = rows
        .iter()
        .map(|c| {
            json!({
                "source": c.source,
                "record": c.record,
                "field": c.field,
                "old": c.old_value,
                "new": c.new_value,
                "observed_on": c.observed_on,
                "snapshot": format!("/snapshot/{}", c.snapshot_id),
            })
        })
        .collect();
    paged("changes", items, page, total, next)
}

pub fn flags_json(rows: &[crate::db::ApiFlagRow], page: u32, total: u32, next: Option<String>) -> String {
    let items = rows
        .iter()
        .map(|f| {
            json!({
                "source": f.source,
                "record": f.record,
                "work": f.work_ref,
                "type": f.kind,
                "rule_version": f.rule_version,
                "value": serde_json::from_str::<Value>(&f.value_json).unwrap_or_default(),
                "status": f.status,
                "raised_on": f.created_on,
                "cleared_on": f.cleared_on,
                "snapshot": format!("/snapshot/{}", f.snapshot_id),
            })
        })
        .collect();
    paged("flags", items, page, total, next)
}

pub fn snapshots_json(rows: &[crate::db::SnapshotListRow], page: u32, total: u32, next: Option<String>) -> String {
    let items = rows
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "source_id": s.source_id,
                "source": s.source,
                "url": s.url,
                "sha256": s.sha256,
                "bytes": s.bytes,
                "fetched_at": s.fetched_at,
                "download": format!("/snapshot/{}", s.id),
            })
        })
        .collect();
    paged("snapshots", items, page, total, next)
}
