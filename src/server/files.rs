//! A device's File objects to save: read whole, never written back.

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;

use super::ApiError;
use super::devices::object_id;
use super::exports::attachment;
use super::session::Shared;
use crate::file_object;
use crate::limits::{self, Limits};

/// The whole file as the device keeps it, saved under the object's name.
pub async fn file(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<impl IntoResponse, ApiError> {
    let file = object_id(object_type, instance)?;
    let client = state.client().await?;
    let limits = limits::read_limits(&client, device)
        .await
        .unwrap_or(Limits::ASSUMED);
    let content = file_object::read_file(&client, device, file, limits.file_octets_per_request())
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    let name = state
        .readings
        .lock()
        .expect("readings lock")
        .get(&device)
        .and_then(|points| points.iter().find(|point| point.object == file))
        .and_then(|point| point.name.clone())
        .unwrap_or_else(|| format!("file-{device}-{instance}"));
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, attachment(&name)),
        ],
        content,
    ))
}
