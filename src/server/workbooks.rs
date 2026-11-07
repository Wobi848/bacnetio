//! The exported lists as Excel files (docs/concept-excel.md): the same tables as the CSV routes,
//! with the column heads and a title line in the reader's language, which the page sends.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::ApiError;
use super::exports::priority_rows;
use super::session::Shared;
use crate::export::{self, Table};
use crate::workbook::{self, Labels};

const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
/// The page sends some 40 heads; more is no page of ours.
const MOST_HEADS: usize = 100;
const LONGEST_HEAD: usize = 80;
const LONGEST_TITLE: usize = 300;

#[derive(Deserialize)]
pub struct WorkbookQuery {
    /// The column heads by their English names, as JSON: `{"Device ID":"Geräte-ID"}`.
    heads: Option<String>,
    /// The line above the header: site, device, time, version.
    title: Option<String>,
    /// The viewer's offset from UTC in minutes, for local times.
    #[serde(default)]
    offset_min: i32,
}

impl WorkbookQuery {
    /// Checked at the border: plain texts of a sane length, nothing else.
    fn labels(&self) -> Result<Labels, ApiError> {
        let heads: HashMap<String, String> = match &self.heads {
            Some(json) => serde_json::from_str(json)
                .map_err(|_| ApiError::bad_request("heads must be a JSON object of texts"))?,
            None => HashMap::new(),
        };
        let too_long = |text: &str, most: usize| text.chars().count() > most;
        if heads.len() > MOST_HEADS
            || heads
                .iter()
                .any(|(key, text)| too_long(key, LONGEST_HEAD) || too_long(text, LONGEST_HEAD))
        {
            return Err(ApiError::bad_request(format!(
                "at most {MOST_HEADS} heads of at most {LONGEST_HEAD} characters"
            )));
        }
        if self
            .title
            .as_deref()
            .is_some_and(|title| too_long(title, LONGEST_TITLE))
        {
            return Err(ApiError::bad_request(format!(
                "a title has at most {LONGEST_TITLE} characters"
            )));
        }
        Ok(Labels {
            heads,
            title: self.title.clone().filter(|title| !title.trim().is_empty()),
        })
    }
}

/// A sheet named by its English name, in the reader's language where the page gave one.
fn named(labels: &Labels, english: &str) -> String {
    labels
        .heads
        .get(english)
        .cloned()
        .unwrap_or_else(|| english.to_string())
}

fn file(
    name: &str,
    sheets: &[(String, &Table)],
    labels: &Labels,
) -> Result<impl IntoResponse + use<>, ApiError> {
    let bytes =
        workbook::xlsx(sheets, labels).map_err(|error| ApiError::internal(error.to_string()))?;
    let disposition = format!("attachment; filename=\"{name}.xlsx\"");
    Ok((
        [
            (header::CONTENT_TYPE, XLSX.to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    ))
}

/// The name a device is known by, for its sheet.
fn device_name(state: &Shared, device: u32) -> String {
    state
        .devices
        .lock()
        .expect("devices lock")
        .iter()
        .find(|found| found.instance == device)
        .and_then(|found| found.details.name.clone())
        .unwrap_or_else(|| format!("#{device}"))
}

pub async fn devices(
    State(state): State<Shared>,
    Query(query): Query<WorkbookQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let labels = query.labels()?;
    let table = export::device_list(&state.devices.lock().expect("devices lock"));
    file("devices", &[(named(&labels, "Devices"), &table)], &labels)
}

pub async fn points(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Query(query): Query<WorkbookQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let labels = query.labels()?;
    let table = {
        let readings = state.readings.lock().expect("readings lock");
        let points = readings
            .get(&device)
            .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
        export::point_list(device, points)
    };
    let name = device_name(&state, device);
    file(&format!("device-{device}"), &[(name, &table)], &labels)
}

pub async fn points_priorities(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Query(query): Query<WorkbookQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let labels = query.labels()?;
    let table = export::point_priorities(device, &priority_rows(&state, device).await?);
    let name = device_name(&state, device);
    file(
        &format!("device-{device}-priorities"),
        &[(name, &table)],
        &labels,
    )
}

/// The device list, then one sheet per device read, by device ID.
pub async fn site(
    State(state): State<Shared>,
    Query(query): Query<WorkbookQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let labels = query.labels()?;
    let devices = export::device_list(&state.devices.lock().expect("devices lock"));
    let mut readings: Vec<(u32, Table)> = state
        .readings
        .lock()
        .expect("readings lock")
        .iter()
        .map(|(device, points)| (*device, export::point_list(*device, points)))
        .collect();
    if readings.is_empty() {
        return Err(ApiError::conflict("read the devices' points first"));
    }
    readings.sort_by_key(|(device, _)| *device);
    let mut sheets = vec![(named(&labels, "Devices"), &devices)];
    sheets.extend(
        readings
            .iter()
            .map(|(device, table)| (device_name(&state, *device), table)),
    );
    file("site", &sheets, &labels)
}

pub async fn log(
    State(state): State<Shared>,
    Query(query): Query<WorkbookQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let labels = query.labels()?;
    let table = export::log_table(
        state.journal.lock().expect("journal lock").entries(),
        query.offset_min,
    );
    file("log", &[(named(&labels, "Log"), &table)], &labels)
}

pub async fn recording(
    State(state): State<Shared>,
    Query(query): Query<WorkbookQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let labels = query.labels()?;
    let table = export::recording_table(
        &state.recording.lock().expect("recording lock"),
        query.offset_min,
    );
    file(
        "recording",
        &[(named(&labels, "Recording"), &table)],
        &labels,
    )
}
