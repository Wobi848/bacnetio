mod common;

use std::time::Duration;

use bacnet_objects::event_log::EventLogObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::constructed::{BACnetEventLogRecord, EventLogDatum, EventNotificationRequest};
use bacnet_types::enums::{EventState, EventType, NotifyType, ObjectType, Segmentation};
use bacnet_types::primitives::{BACnetTimeStamp, Date, ObjectIdentifier, Time};
use bacnetio::{event_log, scan};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn record(minute: u8, to: EventState, notify_type: NotifyType, text: &str) -> BACnetEventLogRecord {
    let time = Time {
        hour: 3,
        minute,
        second: 0,
        hundredths: 0,
    };
    BACnetEventLogRecord {
        date: Date {
            year: 126,
            month: 11,
            day: 1,
            day_of_week: 7,
        },
        time,
        log_datum: EventLogDatum::Notification(EventNotificationRequest {
            process_identifier: 1,
            initiating_device_identifier: ObjectIdentifier::new(ObjectType::DEVICE, 7003).unwrap(),
            event_object_identifier: ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 1).unwrap(),
            timestamp: BACnetTimeStamp::Time(time),
            notification_class: 1,
            priority: 100,
            event_type: EventType::OUT_OF_RANGE,
            message_text: Some(text.to_string()),
            notify_type,
            ack_required: true,
            from_state: EventState::NORMAL,
            to_state: to,
            event_values: None,
        }),
    }
}

#[tokio::test]
async fn reads_the_past_alarms_of_an_event_log() {
    let mut log = EventLogObject::new(1, "Alarm history", 100).unwrap();
    log.add_record(record(
        10,
        EventState::HIGH_LIMIT,
        NotifyType::ALARM,
        "too hot",
    ))
    .unwrap();
    log.add_record(record(
        25,
        EventState::NORMAL,
        NotifyType::ACK_NOTIFICATION,
        "",
    ))
    .unwrap();
    log.add_record(record(
        40,
        EventState::NORMAL,
        NotifyType::ALARM,
        "back to normal",
    ))
    .unwrap();
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(log)];
    let mut device = simulated_device(7003, "AHU-09", 47958, Segmentation::BOTH, objects).await;
    let connection = connect(47959).await;
    let instance = scan::locate(&connection.client, None, Some(at(47958).into()), WAIT)
        .await
        .unwrap();

    let log = ObjectIdentifier::new(ObjectType::EVENT_LOG, 1).unwrap();
    let read = event_log::read_event_log(&connection.client, instance, log, 2).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let read = read.unwrap();
    assert_eq!(read.records.len(), 3, "{read:?}");
    assert_eq!(read.records[0].to, EventState::HIGH_LIMIT);
    assert_eq!(read.records[0].text.as_deref(), Some("too hot"));
    assert!(!read.records[0].acknowledged);
    assert!(read.records[1].acknowledged);
    assert_eq!(read.records[2].time.minute, 40);
}
