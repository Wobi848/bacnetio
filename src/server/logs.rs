//! The device's own logs: Trend Log and Trend Log Multiple records for a chart or a CSV file, and
//! the Event Log's past alarms and events for a list or a CSV file.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use bacnet_types::constructed::BACnetDeviceObjectPropertyReference;
use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::primitives::ObjectIdentifier;
use serde::Serialize;
use serde_json::{Value, json};

use super::ApiError;
use super::devices::object_id;
use super::json::ObjectRef;
use super::session::Shared;
use crate::event_log::{self, EventLog};
use crate::limits::{self, Limits};
use crate::trend::{self, TrendLog};
use crate::{alarms, export, present, property_text};

#[derive(Serialize)]
pub struct ColumnView {
    /// The point's name as read with the point list, else its object: "Supply air temp".
    name: String,
    units: Option<String>,
}

#[derive(Serialize)]
pub struct TrendView {
    /// `["2026-11-01T09:30:00", 21.4]`: the device's local time without zone, then the value, or one
    /// value per column of a Trend Log Multiple.
    records: Vec<Value>,
    /// What a Trend Log Multiple logs, in the order of the values; none for a Trend Log.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    columns: Vec<ColumnView>,
    gaps: bool,
    truncated: bool,
}

/// The records of a Trend Log or Trend Log Multiple object, for a chart.
pub async fn trend(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<Json<TrendView>, ApiError> {
    let (log, columns) = read_trend(&state, device, object_type, instance).await?;
    Ok(Json(TrendView {
        records: log
            .records
            .iter()
            .map(|record| {
                let time =
                    present::date_time_text(&record.date, &record.time).replacen(' ', "T", 1);
                let mut row = vec![json!(time)];
                row.extend(record.values.iter().map(|value| json!(value)));
                Value::Array(row)
            })
            .collect(),
        columns,
        gaps: log.gaps,
        truncated: log.truncated,
    }))
}

pub async fn trend_csv(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<impl IntoResponse, ApiError> {
    let (log, columns) = read_trend(&state, device, object_type, instance).await?;
    let titles = if columns.is_empty() {
        vec![format!(
            "{device} {}",
            present::object_text(object_id(object_type, instance)?)
        )]
    } else {
        columns.into_iter().map(|column| column.name).collect()
    };
    let disposition = format!("attachment; filename=\"trend-{device}-{instance}.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        export::trend_csv(&titles, &log.records),
    ))
}

async fn read_trend(
    state: &Shared,
    device: u32,
    object_type: u32,
    instance: u32,
) -> Result<(TrendLog, Vec<ColumnView>), ApiError> {
    let log = object_id(object_type, instance)?;
    let client = state.client().await?;
    let columns = if log.object_type() == ObjectType::TREND_LOG_MULTIPLE {
        let members = trend::read_members(&client, device, log)
            .await
            .map_err(|error| ApiError::device(format!("{error:#}")))?;
        members
            .iter()
            .map(|member| column(state, device, member))
            .collect()
    } else {
        Vec::new()
    };
    let limits = limits::read_limits(&client, device)
        .await
        .unwrap_or(Limits::ASSUMED);
    let records = trend::read_trend_log(&client, device, log, limits.records_per_request())
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    Ok((records, columns))
}

/// A logged point by the name the point list read for it, with the property when it is not the
/// present value and the device when it is another one.
fn column(state: &Shared, device: u32, member: &BACnetDeviceObjectPropertyReference) -> ColumnView {
    let owner = member
        .device_identifier
        .map_or(device, |owner| owner.instance_number());
    let object = member.object_identifier;
    let property = PropertyIdentifier::from_raw(member.property_identifier);
    let (mut name, units) = point_name(state, owner, object);
    if property != PropertyIdentifier::PRESENT_VALUE {
        name = format!("{name} {}", property_text::property_name(property));
    }
    if owner != device {
        name = format!("{owner} · {name}");
    }
    let units = units.filter(|_| property == PropertyIdentifier::PRESENT_VALUE);
    ColumnView { name, units }
}

/// The object's name as the point list read it, else the object, with its units if known.
fn point_name(state: &Shared, device: u32, object: ObjectIdentifier) -> (String, Option<String>) {
    let readings = crate::locked(&state.readings);
    let point = readings
        .get(&device)
        .and_then(|points| points.iter().find(|point| point.object == object));
    let name = point
        .and_then(|point| point.name.clone())
        .unwrap_or_else(|| present::object_text(object));
    (
        name,
        point.and_then(|point| point.units).map(present::unit_text),
    )
}

#[derive(Serialize)]
pub struct LoggedEventView {
    /// "2026-11-01T09:30:00": the device's local time, without zone.
    time: String,
    #[serde(flatten)]
    object: ObjectRef,
    /// The object's name as the point list read it, else the object.
    name: String,
    from: String,
    to: String,
    acknowledged: bool,
    text: Option<String>,
}

#[derive(Serialize)]
pub struct EventLogView {
    /// Oldest first, as the device keeps them.
    records: Vec<LoggedEventView>,
    gaps: bool,
    truncated: bool,
}

/// The past alarms and events an Event Log object keeps.
pub async fn events(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<Json<EventLogView>, ApiError> {
    let log = read_events(&state, device, object_type, instance).await?;
    Ok(Json(EventLogView {
        records: log
            .records
            .iter()
            .map(|event| LoggedEventView {
                time: present::date_time_text(&event.date, &event.time).replacen(' ', "T", 1),
                object: ObjectRef::new(event.object),
                name: point_name(&state, device, event.object).0,
                from: alarms::state_text(event.from),
                to: alarms::state_text(event.to),
                acknowledged: event.acknowledged,
                text: event.text.clone(),
            })
            .collect(),
        gaps: log.gaps,
        truncated: log.truncated,
    }))
}

pub async fn events_csv(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<impl IntoResponse, ApiError> {
    let log = read_events(&state, device, object_type, instance).await?;
    let disposition = format!("attachment; filename=\"events-{device}-{instance}.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        export::event_log_csv(&log.records, |object| point_name(&state, device, object).0),
    ))
}

async fn read_events(
    state: &Shared,
    device: u32,
    object_type: u32,
    instance: u32,
) -> Result<EventLog, ApiError> {
    let log = object_id(object_type, instance)?;
    let client = state.client().await?;
    let limits = limits::read_limits(&client, device)
        .await
        .unwrap_or(Limits::ASSUMED);
    event_log::read_event_log(&client, device, log, limits.records_per_request())
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))
}
