//! The device's own list of past alarms and events: Event Log objects, read page by page with
//! ReadRange.

use bacnet_services::read_range::LogRecords;
use bacnet_types::constructed::{BACnetEventLogRecord, EventLogDatum};
use bacnet_types::enums::{EventState, NotifyType};
use bacnet_types::primitives::{Date, ObjectIdentifier, Time};

use crate::connection::Client;
use crate::log_buffer::{Buffer, read_buffer};

/// One alarm or event as the device logged it, with its local date and time.
#[derive(Debug, Clone, PartialEq)]
pub struct LoggedEvent {
    pub date: Date,
    pub time: Time,
    /// The object that changed state.
    pub object: ObjectIdentifier,
    pub from: EventState,
    pub to: EventState,
    /// An operator acknowledged it: the device logs the acknowledgement apart, without states.
    pub acknowledged: bool,
    pub text: Option<String>,
}

pub type EventLog = Buffer<LoggedEvent>;

/// All records of an Event Log object, oldest first.
pub async fn read_event_log(
    client: &Client,
    device: u32,
    log: ObjectIdentifier,
    page: u16,
) -> anyhow::Result<EventLog> {
    read_buffer(
        client,
        device,
        log,
        page,
        "event log",
        |records| match records {
            LogRecords::EventLog(records) => {
                Some(records.iter().filter_map(logged_event).collect())
            }
            _ => None,
        },
    )
    .await
}

/// A notification; status changes of the log itself and clock changes are left out.
fn logged_event(record: &BACnetEventLogRecord) -> Option<LoggedEvent> {
    let EventLogDatum::Notification(notification) = &record.log_datum else {
        return None;
    };
    Some(LoggedEvent {
        date: record.date,
        time: record.time,
        object: notification.event_object_identifier,
        from: notification.from_state,
        to: notification.to_state,
        acknowledged: notification.notify_type == NotifyType::ACK_NOTIFICATION,
        text: notification.message_text.clone(),
    })
}
