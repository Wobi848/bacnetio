//! Saving a device's points as reference and comparing the current reading with it.

use std::time::SystemTime;

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use super::ApiError;
use super::session::Shared;
use crate::reference::{self, Change};

#[derive(Serialize)]
pub struct ComparisonView {
    /// When the reference was saved, milliseconds since 1970; `None` when there is none.
    time_ms: Option<u64>,
    changes: Vec<Change>,
}

/// Saves the device's last reading as its reference, replacing an older one.
pub async fn save(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<ComparisonView>, ApiError> {
    let dir = state
        .data_dir
        .clone()
        .ok_or_else(|| ApiError::conflict("bacnetio has no data folder to save to"))?;
    let saved = {
        let readings = crate::locked(&state.readings);
        let points = readings
            .get(&device)
            .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
        reference::from_points(device, SystemTime::now(), points)
    };
    reference::save(&dir, &saved).map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(ComparisonView {
        time_ms: Some(saved.time_ms),
        changes: Vec::new(),
    }))
}

/// What changed between the reference and the device's last reading.
pub async fn compare(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<ComparisonView>, ApiError> {
    let Some(dir) = state.data_dir.clone() else {
        return Ok(Json(ComparisonView {
            time_ms: None,
            changes: Vec::new(),
        }));
    };
    let saved =
        reference::load(&dir, device).map_err(|error| ApiError::internal(error.to_string()))?;
    let Some(saved) = saved else {
        return Ok(Json(ComparisonView {
            time_ms: None,
            changes: Vec::new(),
        }));
    };
    let readings = crate::locked(&state.readings);
    let points = readings
        .get(&device)
        .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
    Ok(Json(ComparisonView {
        time_ms: Some(saved.time_ms),
        changes: reference::compare(&saved, points),
    }))
}
