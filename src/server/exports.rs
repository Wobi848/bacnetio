//! Downloads: device list, point lists and EDE files.

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::ApiError;
use super::session::Shared;
use crate::points::Point;
use crate::{ede, export, object};

/// The devices of the last scan as CSV.
pub async fn devices_csv(State(state): State<Shared>) -> impl IntoResponse {
    let devices = state.devices.lock().expect("devices lock");
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"devices.csv\"",
            ),
        ],
        export::device_list_csv(&devices),
    )
}

/// The point list of the last reading as CSV, for Excel.
pub async fn points_csv(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let readings = state.readings.lock().expect("readings lock");
    let points = readings
        .get(&device)
        .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
    let disposition = format!("attachment; filename=\"device-{device}.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        export::point_list_csv(device, points),
    ))
}

/// `?project=` names the project in an EDE file; the device's name, or none for a site, otherwise.
#[derive(Deserialize)]
pub struct EdeQuery {
    project: Option<String>,
}

fn device_project(state: &Shared, device: u32, query: EdeQuery) -> String {
    query.project.unwrap_or_else(|| {
        state
            .devices
            .lock()
            .expect("devices lock")
            .iter()
            .find(|found| found.instance == device)
            .and_then(|found| found.details.name.clone())
            .unwrap_or_default()
    })
}

/// The point list of the last reading as EDE file, for management systems and other tools.
pub async fn points_ede(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Query(query): Query<EdeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let project = device_project(&state, device, query);
    let readings = state.readings.lock().expect("readings lock");
    let points = readings
        .get(&device)
        .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
    let disposition = format!("attachment; filename=\"device-{device}-ede.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        ede::ede_csv(&project, &[(device, points)], std::time::SystemTime::now()),
    ))
}

/// The EDE file of all devices read since the last start, sorted by device.
pub async fn site_ede(
    State(state): State<Shared>,
    Query(query): Query<EdeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let readings = state.readings.lock().expect("readings lock");
    if readings.is_empty() {
        return Err(ApiError::conflict("read the devices' points first"));
    }
    let mut devices: Vec<(u32, &[Point])> = readings
        .iter()
        .map(|(device, points)| (*device, points.as_slice()))
        .collect();
    devices.sort_by_key(|(device, _)| *device);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"site-ede.csv\"".to_string(),
            ),
        ],
        ede::ede_csv(
            query.project.as_deref().unwrap_or_default(),
            &devices,
            std::time::SystemTime::now(),
        ),
    ))
}

/// The state texts that belong to the device's EDE file.
pub async fn points_ede_states(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let readings = state.readings.lock().expect("readings lock");
    let points = readings
        .get(&device)
        .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
    let disposition = format!("attachment; filename=\"device-{device}-ede-states.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        ede::state_texts_csv(&[(device, points)]),
    ))
}

/// The state texts that belong to the site's EDE file.
pub async fn site_ede_states(State(state): State<Shared>) -> Result<impl IntoResponse, ApiError> {
    let readings = state.readings.lock().expect("readings lock");
    if readings.is_empty() {
        return Err(ApiError::conflict("read the devices' points first"));
    }
    let mut devices: Vec<(u32, &[Point])> = readings
        .iter()
        .map(|(device, points)| (*device, points.as_slice()))
        .collect();
    devices.sort_by_key(|(device, _)| *device);
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"site-ede-states.csv\"".to_string(),
            ),
        ],
        ede::state_texts_csv(&devices),
    ))
}

/// The point list of the last reading as CSV, with the priority arrays read from the device now,
/// one object after the other, so that a small controller is not flooded.
pub async fn points_priorities_csv(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<impl IntoResponse, ApiError> {
    let points = state
        .readings
        .lock()
        .expect("readings lock")
        .get(&device)
        .cloned()
        .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
    let client = state.client().await?;
    let mut rows = Vec::with_capacity(points.len());
    for point in points {
        let priorities = if object::commandable_kind(point.object.object_type()) {
            object::read_priorities(&client, device, point.object).await
        } else {
            Ok(None)
        };
        rows.push((point, priorities));
    }
    let disposition = format!("attachment; filename=\"device-{device}-priorities.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        export::point_priorities_csv(device, &rows),
    ))
}
