//! A device's active alarms and faults (GetEventInformation): objects whose event state is not
//! normal, or whose transitions are not acknowledged yet.

use anyhow::Context;
use bacnet_types::bitstring::EventTransitionBits;
use bacnet_types::enums::EventState;
use bacnet_types::primitives::{BACnetTimeStamp, ObjectIdentifier};

use crate::connection::Client;

/// Pages asked for at most; a device lists its alarms in pages, each continuing after the last.
const MOST_PAGES: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct Alarm {
    pub object: ObjectIdentifier,
    pub state: EventState,
    /// A transition (to off-normal, fault or normal) waits for an acknowledgement.
    pub unacknowledged: bool,
    /// When the device entered this state, which an acknowledgement refers to. Older devices that
    /// only know GetAlarmSummary do not tell it.
    pub timestamp: Option<BACnetTimeStamp>,
}

/// The device's alarms: by GetEventInformation, or by GetAlarmSummary where it knows only that.
pub async fn read_alarms(client: &Client, device: u32) -> anyhow::Result<Vec<Alarm>> {
    let first = match read_event_information(client, device).await {
        Ok(alarms) => return Ok(alarms),
        Err(error) => error,
    };
    match client.get_alarm_summary_from_device(device).await {
        Ok(summary) => Ok(summary
            .entries
            .iter()
            .map(|entry| Alarm {
                object: entry.object_identifier,
                state: entry.alarm_state,
                unacknowledged: entry.acknowledged_transitions != EventTransitionBits::all(),
                timestamp: None,
            })
            .collect()),
        Err(_) => Err(first),
    }
}

async fn read_event_information(client: &Client, device: u32) -> anyhow::Result<Vec<Alarm>> {
    let mut alarms = Vec::new();
    let mut after = None;
    for _ in 0..MOST_PAGES {
        let page = client
            .get_event_information_from_device(device, after)
            .await
            .context("cannot read the active alarms (GetEventInformation)")?;
        alarms.extend(page.list_of_event_summaries.iter().map(|summary| Alarm {
            object: summary.object_identifier,
            state: summary.event_state,
            unacknowledged: summary.acknowledged_transitions != EventTransitionBits::all(),
            timestamp: Some(
                summary.event_timestamps[transition_index(summary.event_state)].clone(),
            ),
        }));
        after = alarms.last().map(|alarm| alarm.object);
        if !page.more_events || after.is_none() {
            break;
        }
    }
    Ok(alarms)
}

/// Where a state's time stands in the device's list: to off-normal, to fault, to normal.
fn transition_index(state: EventState) -> usize {
    if state == EventState::FAULT {
        1
    } else if state == EventState::NORMAL {
        2
    } else {
        0
    }
}

/// "high limit", "fault", "offnormal": the event state in words.
pub fn state_text(state: EventState) -> String {
    match state.name() {
        Some(name) => name.to_lowercase().replace('_', " "),
        None => format!("state {}", state.to_raw()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_states_in_words() {
        assert_eq!(state_text(EventState::HIGH_LIMIT), "high limit");
        assert_eq!(state_text(EventState::FAULT), "fault");
    }
}
