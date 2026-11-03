//! Commands to a device beyond writing values: restart, communication, acknowledging an alarm, and
//! asking the whole network who has an object of a name.

use std::time::Duration;

use anyhow::Context;
use bacnet_services::alarm_event::AcknowledgeAlarmRequest;
use bacnet_services::who_has::WhoHasObject;
use bacnet_types::enums::{EnableDisable, ReinitializedState};
use bacnet_types::error::Error;
use bacnet_types::primitives::{BACnetTimeStamp, Date, ObjectIdentifier, Time};

use crate::alarms::Alarm;
use crate::connection::Client;

/// The process that acknowledges on the device's books: bacnetio has just the one.
const ACKNOWLEDGING_PROCESS: u32 = 1;
/// How long answers to Who-Has are collected.
const WHO_HAS_WAIT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Restart {
    Warm,
    Cold,
}

pub async fn restart(
    client: &Client,
    device: u32,
    kind: Restart,
    password: Option<String>,
) -> Result<(), Error> {
    let state = match kind {
        Restart::Warm => ReinitializedState::WARMSTART,
        Restart::Cold => ReinitializedState::COLDSTART,
    };
    client
        .reinitialize_device_from_device(device, state, password)
        .await
}

/// Stops the device's communication for `minutes`, or for good with `None`, or lets it talk again.
pub async fn communication(
    client: &Client,
    device: u32,
    enable: bool,
    minutes: Option<u16>,
    password: Option<String>,
) -> Result<(), Error> {
    let state = if enable {
        EnableDisable::ENABLE
    } else {
        EnableDisable::DISABLE
    };
    client
        .device_communication_control_from_device(device, state, minutes, password)
        .await
}

/// `timestamp` is when the device entered the state, as it told it.
pub async fn acknowledge(
    client: &Client,
    device: u32,
    alarm: &Alarm,
    timestamp: BACnetTimeStamp,
    date: Date,
    time: Time,
) -> Result<(), Error> {
    let request = AcknowledgeAlarmRequest {
        acknowledging_process_identifier: ACKNOWLEDGING_PROCESS,
        event_object_identifier: alarm.object,
        event_state_acknowledged: alarm.state,
        timestamp,
        acknowledgment_source: "bacnetio".to_string(),
        time_of_acknowledgment: BACnetTimeStamp::DateTime { date, time },
    };
    client
        .acknowledge_alarm_request_from_device(device, &request)
        .await
}

/// Who on the network has an object of this name; devices need not be read for this.
pub async fn who_has(
    client: &Client,
    name: &str,
) -> anyhow::Result<Vec<(u32, ObjectIdentifier, String)>> {
    let replies = client
        .who_has_replies(WhoHasObject::Name(name.to_string()), None, WHO_HAS_WAIT)
        .await
        .context("cannot send Who-Has")?;
    Ok(replies
        .into_iter()
        .map(|reply| {
            (
                reply.i_have.device_identifier.instance_number(),
                reply.i_have.object_identifier,
                reply.i_have.object_name,
            )
        })
        .collect())
}
