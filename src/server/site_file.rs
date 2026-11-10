//! A site as a file (docs/concept-explorer.md): handed out to keep or to pass to a colleague, and
//! brought back in. A file comes from outside, so its form is checked before it is kept.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::Value;

use super::ApiError;
use super::session::Shared;
use super::site::{FINISHED, MOST_DEVICES, Site, file_stem, kept_site, name_beside, save, show};

#[derive(Deserialize)]
pub struct ExportQuery {
    /// A kept site; without it the site shown now.
    name: Option<String>,
}

/// A site as a file, to keep it or to hand it to a colleague.
pub async fn export(
    State(state): State<Shared>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let shown = crate::locked(&state.site).clone();
    let site = match query.name {
        Some(name) if shown.as_ref().is_none_or(|site| site.name != name) => state
            .data_dir
            .as_deref()
            .and_then(|dir| kept_site(dir, &name))
            .ok_or_else(|| ApiError::bad_request(format!("no site named {name}")))?,
        _ => shown.ok_or_else(|| ApiError::conflict("search for devices first"))?,
    };
    let disposition = format!(
        "attachment; filename=\"bacnetio-site-{}.json\"",
        file_stem(&site.name)
    );
    Ok(([(header::CONTENT_DISPOSITION, disposition)], Json(site)))
}

/// The numbers of a device in a site file; everything else in it is shown as escaped text.
#[derive(Deserialize)]
struct CheckedDevice {
    #[serde(rename = "instance")]
    _instance: u32,
    #[serde(rename = "network")]
    _network: Option<u16>,
    #[serde(default, rename = "objects")]
    _objects: Option<CheckedObjects>,
}

#[derive(Deserialize)]
struct CheckedObjects {
    #[serde(rename = "read")]
    _read: usize,
    #[serde(rename = "total")]
    _total: Option<usize>,
    #[serde(default, rename = "types")]
    _types: BTreeMap<u32, usize>,
}

#[derive(Deserialize)]
struct CheckedRouter {
    #[serde(rename = "networks")]
    _networks: Vec<u16>,
}

/// What the interface reads as text or walks as a list. A number in `value` or `status` throws in
/// the page (`value.endsWith`, `status.includes`) and takes the whole redraw with it.
#[derive(Deserialize)]
struct CheckedPoint {
    #[serde(rename = "type")]
    _object_type: u32,
    #[serde(rename = "instance")]
    _instance: u32,
    #[serde(default, rename = "value")]
    _value: String,
    #[serde(default, rename = "status")]
    _status: String,
    #[serde(default, rename = "units")]
    _units: Option<String>,
    #[serde(default, rename = "state_texts")]
    _state_texts: Vec<String>,
    #[serde(default, rename = "binary_texts")]
    _binary_texts: Vec<String>,
    #[serde(default, rename = "notification_class")]
    _notification_class: Option<u32>,
}

#[derive(Deserialize)]
struct CheckedPoints {
    #[serde(rename = "points")]
    _points: Vec<CheckedPoint>,
}

/// A file from outside is checked for what the interface puts into its pages as it is, numbers
/// above all: a script in place of a device ID would run in the page that can write to the plant.
/// Beyond that it must not be able to wedge the page: a search that never ends, a value the page
/// reads as text, or more devices than the tree can draw.
fn well_formed(site: &Site) -> bool {
    fn fits<T: serde::de::DeserializeOwned>(value: &Value) -> bool {
        serde_json::from_value::<T>(value.clone()).is_ok()
    }
    let Some(scan) = &site.scan else {
        return false;
    };
    // a kept site is a finished search; "running" would set the page polling for a scan that is
    // not there, with no way out of "Searching …"
    scan["state"] == FINISHED
        && scan["devices"]
            .as_array()
            .is_some_and(|devices| devices.len() <= MOST_DEVICES)
        && fits::<Vec<CheckedDevice>>(&scan["devices"])
        && (scan["routers"].is_null() || fits::<Vec<CheckedRouter>>(&scan["routers"]))
        && site
            .points
            .iter()
            .all(|(device, points)| device.parse::<u32>().is_ok() && fits::<CheckedPoints>(points))
}

/// A site file brought back in. It comes from outside, so its form is checked before it is kept.
pub async fn import(
    State(state): State<Shared>,
    Json(mut site): Json<Site>,
) -> Result<StatusCode, ApiError> {
    if !well_formed(&site) {
        return Err(ApiError::bad_request("this is not a bacnetio site file"));
    }
    if site.name.is_empty() {
        site.name = "imported".to_string();
    }
    if let Some(dir) = &state.data_dir {
        site.name = name_beside(dir, &site);
        save(dir, &site).map_err(|error| ApiError::internal(error.to_string()))?;
    }
    show(&state, site);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_file_with_anything_but_numbers_where_numbers_belong_is_refused() {
        let site = |scan: Value, points: Value| Site {
            scan: Some(scan),
            points: serde_json::from_value(points).unwrap(),
            ..Site::default()
        };
        let device = serde_json::json!({ "instance": 21, "network": null, "name": "<b>any text</b>",
            "objects": { "read": 3, "total": 3, "done": true, "error": null, "types": { "0": 2 } } });
        let good =
            serde_json::json!({ "21": { "points": [{ "type": 0, "instance": 1, "name": "x" }] } });
        assert!(well_formed(&site(
            serde_json::json!({ "state": "done", "devices": [device] }),
            good.clone()
        )));
        let script = "<img src=x onerror=alert(1)>";
        for bad in [
            serde_json::json!({ "state": "done", "devices": "none" }),
            serde_json::json!({ "state": "done", "devices": [{ "instance": script, "network": null }] }),
            serde_json::json!({ "state": "done", "devices": [{ "instance": 1, "network": script }] }),
            serde_json::json!({ "state": "done", "devices": [{ "instance": 1, "network": null, "objects": { "read": 1, "total": script } }] }),
            serde_json::json!({ "state": "done", "devices": [], "routers": [{ "networks": [script] }] }),
        ] {
            assert!(!well_formed(&site(bad, good.clone())));
        }
        let scan = serde_json::json!({ "state": "done", "devices": [] });
        assert!(!well_formed(&site(
            scan.clone(),
            serde_json::json!({ "21": { "points": [{ "type": script, "instance": 1 }] } })
        )));
        assert!(!well_formed(&site(
            scan,
            serde_json::json!({ script: { "points": [] } })
        )));
        assert!(!well_formed(&Site::default()));
    }

    /// A file that would leave the page unusable: a search that never ends, a value the page reads
    /// as text, or more devices than the tree can draw.
    #[test]
    fn a_site_file_that_would_wedge_the_page_is_refused() {
        let site = |scan: Value, points: Value| Site {
            scan: Some(scan),
            points: serde_json::from_value(points).unwrap(),
            ..Site::default()
        };
        let empty = serde_json::json!({ "21": { "points": [] } });
        for state in ["running", "idle", "failed"] {
            let scan = serde_json::json!({ "state": state, "devices": [] });
            assert!(
                !well_formed(&site(scan, empty.clone())),
                "a site file with the state {state} was taken in"
            );
        }
        let done = serde_json::json!({ "state": "done", "devices": [] });
        for bad in [
            serde_json::json!({ "type": 0, "instance": 1, "value": 21.5 }),
            serde_json::json!({ "type": 0, "instance": 1, "status": ["fault"] }),
            serde_json::json!({ "type": 0, "instance": 1, "units": 62 }),
            serde_json::json!({ "type": 0, "instance": 1, "state_texts": "Off" }),
        ] {
            let points = serde_json::json!({ "21": { "points": [bad] } });
            assert!(!well_formed(&site(done.clone(), points)));
        }
        let many: Vec<Value> = (0..=MOST_DEVICES)
            .map(|instance| serde_json::json!({ "instance": instance, "network": null }))
            .collect();
        let crowd = serde_json::json!({ "state": "done", "devices": many });
        assert!(!well_formed(&site(crowd, empty)));
    }
}
