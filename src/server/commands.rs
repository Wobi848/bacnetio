//! Commands to a device from the interface: restart, communication, acknowledging an alarm, schedules
//! and calendars, and the question to the whole network who has an object.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::ApiError;
use super::devices::object_id;
use super::session::{EventView, Shared};
use super::writing::journaled;
use crate::commands::{self, Restart};
use crate::present::object_text;
use crate::write::Action;
use crate::{alarms, schedule};
use bacnet_types::enums::ObjectType;

/// No password stays no password: an empty field is not an empty password.
fn password(text: Option<String>) -> Option<String> {
    text.filter(|password| !password.is_empty())
}

#[derive(Deserialize)]
pub struct RestartRequest {
    /// "warm" or "cold"
    kind: String,
    password: Option<String>,
}

pub async fn restart(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Json(request): Json<RestartRequest>,
) -> Result<StatusCode, ApiError> {
    let kind = match request.kind.as_str() {
        "warm" => Restart::Warm,
        "cold" => Restart::Cold,
        other => {
            return Err(ApiError::bad_request(format!(
                "{other} is neither warm nor cold"
            )));
        }
    };
    let object = object_id(ObjectType::DEVICE.to_raw(), device)?;
    let action = Action::Command {
        text: format!("{} start", request.kind),
        password: password(request.password),
        kind: crate::write::Command::Restart(kind),
    };
    journaled(&state, device, object, None, action).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Why a device refuses a stop of all its communication, and what works instead.
const WITHDRAWN_STOP: &str =
    "the device no longer takes this stop (withdrawn in ASHRAE 135-2020): stop its sending instead";

#[derive(Deserialize)]
pub struct CommunicationRequest {
    enable: bool,
    /// A stop of only what the device sends by itself; it still answers. Without it, a stop of all
    /// of it, which devices after ASHRAE 135-2020 refuse.
    #[serde(default)]
    sending_only: bool,
    /// How long it stays stopped; none is until told otherwise.
    minutes: Option<u16>,
    password: Option<String>,
}

pub async fn communication(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Json(request): Json<CommunicationRequest>,
) -> Result<StatusCode, ApiError> {
    if request.minutes == Some(0) {
        return Err(ApiError::bad_request("a stop lasts at least one minute"));
    }
    let object = object_id(ObjectType::DEVICE.to_raw(), device)?;
    let stopped = if request.sending_only {
        "sending off"
    } else {
        "communication off"
    };
    let text = match (request.enable, request.minutes) {
        (true, _) => "communication on".to_string(),
        (false, Some(minutes)) => format!("{stopped} for {minutes} min"),
        (false, None) => stopped.to_string(),
    };
    let stops_all = !request.enable && !request.sending_only;
    let action = Action::Command {
        text,
        password: password(request.password),
        kind: crate::write::Command::Communication {
            enable: request.enable,
            sending_only: request.sending_only,
            minutes: request.minutes,
        },
    };
    journaled(&state, device, object, None, action)
        .await
        .map_err(|error| {
            if stops_all && error.message.contains("service-request-denied") {
                ApiError::device(WITHDRAWN_STOP)
            } else {
                error
            }
        })?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct AcknowledgeRequest {
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
    /// The viewer's local time, which devices keep too.
    date: String,
    time: String,
    weekday: u8,
}

/// Acknowledges what the device reports for the object now, read just before.
pub async fn acknowledge(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Json(request): Json<AcknowledgeRequest>,
) -> Result<StatusCode, ApiError> {
    let object = object_id(request.object_type, request.instance)?;
    let (date, time) = crate::typed::parse_clock(&request.date, &request.time, request.weekday)
        .map_err(ApiError::bad_request)?;
    let client = state.client().await?;
    let alarm = alarms::read_alarms(&client, device)
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?
        .into_iter()
        .find(|alarm| alarm.object == object)
        .ok_or_else(|| {
            ApiError::conflict(format!("{} is not in alarm any more", object_text(object)))
        })?;
    let timestamp = alarm.timestamp.clone().ok_or_else(|| {
        ApiError::conflict(
            "the device does not tell when the alarm began, so it cannot be acknowledged from here",
        )
    })?;
    let result = commands::acknowledge(&client, device, &alarm, timestamp, date, time).await;
    super::writing::record_command(
        &state,
        device,
        object,
        format!("acknowledged {}", alarms::state_text(alarm.state)),
        result.as_ref().err().map(ToString::to_string),
    );
    result.map_err(|error| ApiError::device(error.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct ChangeView {
    time: String,
    value: String,
    /// As typed to change the day, nothing rounded: "06:30:30 21.456".
    exact: String,
}

#[derive(Serialize)]
pub struct ExceptionView {
    when: String,
    priority: u64,
    changes: Vec<ChangeView>,
}

#[derive(Serialize)]
pub struct ScheduleView {
    week: Vec<Vec<ChangeView>>,
    exceptions: Vec<ExceptionView>,
}

fn change_views(changes: Vec<schedule::Change>) -> Vec<ChangeView> {
    changes
        .into_iter()
        .map(|change| ChangeView {
            time: change.time,
            value: change.value,
            exact: change.exact,
        })
        .collect()
}

pub async fn schedule(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<Json<ScheduleView>, ApiError> {
    let object = object_id(object_type, instance)?;
    let client = state.client().await?;
    let found = schedule::read_schedule(&client, device, object)
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    Ok(Json(ScheduleView {
        week: found.week.into_iter().map(change_views).collect(),
        exceptions: found
            .exceptions
            .into_iter()
            .map(|exception| ExceptionView {
                when: exception.when,
                priority: exception.priority,
                changes: change_views(exception.changes),
            })
            .collect(),
    }))
}

#[derive(Deserialize)]
pub struct DayRequest {
    /// 0 is Monday.
    day: u32,
    /// "06:00 21.5; 18:00 17", empty for no change on that day.
    changes: String,
}

/// Writes one day of a schedule's week, logged like every change.
pub async fn write_day(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
    Json(request): Json<DayRequest>,
) -> Result<StatusCode, ApiError> {
    if request.day > 6 {
        return Err(ApiError::bad_request("a week has the days 0 to 6"));
    }
    let object = object_id(object_type, instance)?;
    let client = state.client().await?;
    let result = schedule::write_day(&client, device, object, request.day, &request.changes).await;
    // the day as it was, so the log tells how to put it back
    let before = match &result {
        Ok(Some(before)) if before.is_empty() => " (before: no changes)".to_string(),
        Ok(Some(before)) => format!(" (before: {before})"),
        _ => String::new(),
    };
    super::writing::record_command(
        &state,
        device,
        object,
        format!(
            "day {} of the week: {}{before}",
            request.day + 1,
            request.changes
        ),
        result.as_ref().err().map(|error| format!("{error:#}")),
    );
    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        // the device's refusal is the device's; anything else was typed wrong
        Err(error) if format!("{error:#}").starts_with("the device refused") => {
            Err(ApiError::device(format!("{error:#}")))
        }
        Err(error) if format!("{error:#}").starts_with("cannot tell") => {
            Err(ApiError::conflict(format!("{error:#}")))
        }
        Err(error) => Err(ApiError::bad_request(format!("{error:#}"))),
    }
}

pub async fn calendar(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<Json<Vec<String>>, ApiError> {
    let object = object_id(object_type, instance)?;
    let client = state.client().await?;
    schedule::read_calendar(&client, device, object)
        .await
        .map(Json)
        .map_err(|error| ApiError::device(format!("{error:#}")))
}

#[derive(Deserialize)]
pub struct WhoHasRequest {
    name: String,
}

#[derive(Serialize)]
pub struct WhoHasView {
    device: u32,
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
    name: String,
}

pub async fn who_has(
    State(state): State<Shared>,
    Json(request): Json<WhoHasRequest>,
) -> Result<Json<Vec<WhoHasView>>, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("give a name to ask for"));
    }
    let client = state.client().await?;
    let found = commands::who_has(&client, name)
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    Ok(Json(
        found
            .into_iter()
            .map(|(device, object, name)| WhoHasView {
                device,
                object_type: object.object_type().to_raw(),
                instance: object.instance_number(),
                name,
            })
            .collect(),
    ))
}

/// Alarm messages the devices sent to this computer since the start, the latest last.
pub async fn events(State(state): State<Shared>) -> Json<Vec<EventView>> {
    Json(crate::locked(&state.events).iter().cloned().collect())
}
