//! Writing to the plant: values, releases and the device clock, each logged in the journal, and
//! the values still held.

use std::net::SocketAddrV4;
use std::time::{Duration, SystemTime};

use axum::Json;
use axum::extract::{Path, State};
use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use serde::Deserialize;

use super::ApiError;
use super::devices::{object_id, object_view};
use super::json::{EntryView, ObjectView};
use super::session::Shared;
use crate::connection::socket_address;
use crate::object::PRIORITIES;
use crate::property_text;
use crate::scan;
use crate::storage;
use crate::write::{self, Action, Entry};

#[derive(Deserialize)]
pub struct ClockRequest {
    /// The viewer's local date and time: "2026-11-01", "09:30:05", weekday 1 (Monday) to 7.
    date: String,
    time: String,
    weekday: u8,
}

/// Sets the device's clock (TimeSynchronization, unconfirmed).
pub async fn set_clock(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Json(request): Json<ClockRequest>,
) -> Result<axum::http::StatusCode, ApiError> {
    let (date, time) = write::parse_clock(&request.date, &request.time, request.weekday)
        .map_err(ApiError::bad_request)?;
    let object = object_id(ObjectType::DEVICE.to_raw(), device)?;
    journaled(
        &state,
        device,
        object,
        None,
        Action::SetClock { date, time },
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct WriteRequest {
    /// As typed: "21.5", "active", "2".
    value: String,
    /// 1 to 16; missing for objects without priority array.
    priority: Option<u8>,
}

pub async fn write_value(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
    Json(request): Json<WriteRequest>,
) -> Result<Json<ObjectView>, ApiError> {
    let object = object_id(object_type, instance)?;
    let value =
        write::parse_value(object.object_type(), &request.value).map_err(ApiError::bad_request)?;
    let priority = request.priority.map(checked_priority).transpose()?;
    perform(&state, device, object, priority, Action::Write(value)).await
}

#[derive(Deserialize)]
pub struct PropertyRequest {
    /// The number of the property, as `GET .../properties` tells it.
    property_id: u32,
    /// The kind of value it holds, as `GET .../properties` tells it.
    data_type: String,
    value: String,
}

/// Writes any property that holds a plain value; whether it may change, the device decides.
pub async fn write_property(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
    Json(request): Json<PropertyRequest>,
) -> Result<Json<ObjectView>, ApiError> {
    let object = object_id(object_type, instance)?;
    let property = PropertyIdentifier::from_raw(request.property_id);
    if write::NOT_WRITABLE.contains(&property) {
        let name = property_text::property_name(property);
        return Err(ApiError::bad_request(format!(
            "{name} cannot be changed here"
        )));
    }
    let value = write::parse_property_value(&request.data_type, &request.value)
        .map_err(ApiError::bad_request)?;
    let renamed = match &value {
        PropertyValue::CharacterString(name)
            if property == PropertyIdentifier::OBJECT_NAME
                && object.object_type() == ObjectType::DEVICE =>
        {
            Some(name.clone())
        }
        _ => None,
    };
    let view = perform(
        &state,
        device,
        object,
        None,
        Action::WriteProperty { property, value },
    )
    .await?;
    // the device list, the site memory and the exports name the device by what the search read
    if let Some(name) = renamed {
        for known in state.devices.lock().expect("devices lock").iter_mut() {
            if known.instance == device {
                known.details.name = Some(name.clone());
            }
        }
    }
    Ok(view)
}

/// How long a device with the wanted ID may take to answer before the ID counts as free.
const ID_CHECK_WAIT: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
pub struct InstanceRequest {
    instance: u32,
}

/// Gives the device a new device ID, where it allows that. Afterwards it answers under the new ID
/// only, so the interface searches again. An ID another device has would make both unreachable.
pub async fn change_instance(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Json(request): Json<InstanceRequest>,
) -> Result<axum::http::StatusCode, ApiError> {
    let new_id = ObjectIdentifier::new(ObjectType::DEVICE, request.instance)
        .ok()
        .filter(|_| request.instance <= scan::MAX_INSTANCE)
        .ok_or_else(|| {
            ApiError::bad_request(format!("a device ID goes from 0 to {}", scan::MAX_INSTANCE))
        })?;
    if request.instance == device {
        return Err(ApiError::bad_request(format!(
            "{device} is already the ID of this device"
        )));
    }
    let holding = state
        .journal
        .lock()
        .expect("journal lock")
        .held()
        .iter()
        .any(|entry| entry.device == device);
    if holding {
        // under a new ID they could no longer be released
        return Err(ApiError::bad_request(
            "release the values you hold on this device first",
        ));
    }
    let known = state
        .devices
        .lock()
        .expect("devices lock")
        .iter()
        .any(|known| known.instance == request.instance);
    let client = state.client().await?;
    // the last search may be old or from a remembered site: the network itself has the last word
    let answers = scan::locate(&client, Some(request.instance), None, ID_CHECK_WAIT)
        .await
        .is_ok();
    if known || answers {
        return Err(ApiError::bad_request(format!(
            "another device already has the ID {}",
            request.instance
        )));
    }
    let object = object_id(ObjectType::DEVICE.to_raw(), device)?;
    let action = || Action::WriteProperty {
        property: PropertyIdentifier::OBJECT_IDENTIFIER,
        value: PropertyValue::ObjectIdentifier(new_id),
    };
    match journaled(&state, device, object, None, action()).await {
        // a device set up with an ID beyond the 22 bits BACnet has answers under a cut-off one it
        // does not know as its own; the wildcard instance means "this device" to many
        Err(error) if error.message.contains("unknown-object") => {
            let this_device = object_id(ObjectType::DEVICE.to_raw(), scan::WILDCARD_INSTANCE)?;
            journaled(&state, device, this_device, None, action()).await?;
        }
        result => result?,
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReleaseRequest {
    priority: u8,
}

pub async fn release(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
    Json(request): Json<ReleaseRequest>,
) -> Result<Json<ObjectView>, ApiError> {
    let object = object_id(object_type, instance)?;
    let priority = checked_priority(request.priority)?;
    perform(&state, device, object, Some(priority), Action::Release).await
}

/// Performs and journals the action, then reads the object again to show its effect.
async fn perform(
    state: &Shared,
    device: u32,
    object: ObjectIdentifier,
    priority: Option<u8>,
    action: Action,
) -> Result<Json<ObjectView>, ApiError> {
    journaled(state, device, object, priority, action).await?;
    Ok(Json(object_view(state, device, object).await?))
}

/// Performs the action and writes it to the journal, refused or not: every change to the plant
/// is logged.
pub(super) async fn journaled(
    state: &Shared,
    device: u32,
    object: ObjectIdentifier,
    priority: Option<u8>,
    action: Action,
) -> Result<(), ApiError> {
    let address = held_address(state, device);
    let client = state.client_for(device, address).await?;
    // the device whose details the interface shows: the first in the list with this ID
    let listed = state
        .devices
        .lock()
        .expect("devices lock")
        .iter()
        .find(|known| known.instance == device)
        .cloned();
    if let Some(listed) = listed {
        scan::aim_at(&client, &listed)
            .await
            .map_err(|error| ApiError::device(format!("{error:#}")))?;
    }
    let address = client
        .get_device(device)
        .await
        .filter(|found| found.source_network.is_none())
        .and_then(|found| socket_address(&found.mac_address));
    let result = write::perform(&client, device, object, priority, &action).await;
    let error = result.err().map(|error| error.to_string());
    if error.is_none() {
        state
            .watch
            .lock()
            .expect("watch lock")
            .forget_report(device, object);
    }
    let entry = Entry {
        time: SystemTime::now(),
        device,
        address,
        object,
        priority,
        action,
        error: error.clone(),
    };
    if let Some(path) = &state.journal_file()
        && let Err(reason) = storage::append(path, &entry)
    {
        // the write itself happened; the log on disk is a safety net, so report and carry on
        eprintln!(
            "Warning: cannot save the write log to {}: {reason}",
            path.display()
        );
    }
    state.journal.lock().expect("journal lock").record(entry);
    match error {
        Some(reason) => Err(ApiError::device(format!("the device refused: {reason}"))),
        None => Ok(()),
    }
}

/// Where the device was reached when bacnetio last wrote to it, from the journal.
fn held_address(state: &Shared, device: u32) -> Option<SocketAddrV4> {
    let journal = state.journal.lock().expect("journal lock");
    journal
        .entries()
        .iter()
        .rev()
        .find(|entry| entry.device == device)
        .and_then(|entry| entry.address)
}

/// Values bacnetio wrote at a priority and has not released yet.
pub async fn held(State(state): State<Shared>) -> Json<Vec<EntryView>> {
    let journal = state.journal.lock().expect("journal lock");
    Json(journal.held().into_iter().map(EntryView::new).collect())
}

/// Every write and release of this session, oldest first.
pub async fn log(State(state): State<Shared>) -> Json<Vec<EntryView>> {
    let journal = state.journal.lock().expect("journal lock");
    Json(journal.entries().iter().map(EntryView::new).collect())
}

fn checked_priority(priority: u8) -> Result<u8, ApiError> {
    if (1..=PRIORITIES).contains(&priority) {
        Ok(priority)
    } else {
        Err(ApiError::bad_request(format!(
            "priority {priority} is not between 1 and {PRIORITIES}"
        )))
    }
}

/// Logs a command that was carried out elsewhere.
pub(super) fn record_command(
    state: &Shared,
    device: u32,
    object: ObjectIdentifier,
    text: String,
    error: Option<String>,
) {
    let entry = Entry {
        time: SystemTime::now(),
        device,
        address: None,
        object,
        priority: None,
        action: Action::Noted(text),
        error,
    };
    if let Some(path) = &state.journal_file()
        && let Err(reason) = storage::append(path, &entry)
    {
        eprintln!("Warning: cannot save the write log: {reason}");
    }
    state.journal.lock().expect("journal lock").record(entry);
}
