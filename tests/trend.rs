mod common;

use std::time::Duration;

use bacnet_objects::traits::BACnetObject;
use bacnet_objects::trend::{TrendLogMultipleObject, TrendLogObject};
use bacnet_types::constructed::{
    BACnetDeviceObjectPropertyReference, BACnetLogMultipleRecord, BACnetLogRecord, LogData,
    LogDatum, LogValue,
};
use bacnet_types::enums::{ObjectType, Segmentation};
use bacnet_types::primitives::{Date, ObjectIdentifier, Time};
use bacnetio::{scan, trend};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn day() -> Date {
    Date {
        year: 126,
        month: 11,
        day: 1,
        day_of_week: 7,
    }
}

fn minute(minute: u32) -> Time {
    Time {
        hour: (8 + minute / 60) as u8,
        minute: (minute % 60) as u8,
        second: 0,
        hundredths: 0,
    }
}

fn supply_air_log(records: u32) -> TrendLogObject {
    let mut log = TrendLogObject::new(1, "Supply air temp log", 1000).unwrap();
    for minute_of_log in 0..records {
        log.add_record(BACnetLogRecord {
            date: day(),
            time: minute(minute_of_log),
            log_datum: LogDatum::RealValue(20.0 + minute_of_log as f32 / 10.0),
            status_flags: None,
        })
        .unwrap();
    }
    log
}

#[tokio::test]
async fn reads_a_trend_log_over_several_pages() {
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(supply_air_log(120))];
    let mut device = simulated_device(7001, "AHU-07", 47935, Segmentation::BOTH, objects).await;
    let connection = connect(47936).await;
    let instance = scan::locate(&connection.client, None, Some(at(47935).into()), WAIT)
        .await
        .unwrap();

    let log = ObjectIdentifier::new(ObjectType::TREND_LOG, 1).unwrap();
    let read = trend::read_trend_log(&connection.client, instance, log, 50).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let read = read.unwrap();
    assert_eq!(read.records.len(), 120);
    assert!(!read.truncated);
    assert_eq!(read.records[0].values, vec![Some(20.0)]);
    assert_eq!(read.records[119].values, vec![Some(31.9)]);
    assert_eq!(
        (read.records[119].time.hour, read.records[119].time.minute),
        (9, 59)
    );
}

#[tokio::test]
async fn reads_a_trend_log_multiple_with_the_points_it_logs() {
    let supply = ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 1).unwrap();
    let fan = ObjectIdentifier::new(ObjectType::BINARY_OUTPUT, 1).unwrap();
    let mut log = TrendLogMultipleObject::new(1, "AHU log", 1000).unwrap();
    for member in [supply, fan] {
        log.add_property_reference(BACnetDeviceObjectPropertyReference::new_local(member, 85))
            .unwrap();
    }
    for at in 0..80 {
        log.add_record(BACnetLogMultipleRecord {
            date: day(),
            time: minute(at),
            log_data: LogData::Values(vec![
                LogValue::RealValue(20.0 + at as f32 / 10.0),
                LogValue::EnumValue(u64::from(at % 2)),
            ]),
        })
        .unwrap();
    }
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(log)];
    let mut device = simulated_device(7002, "AHU-08", 47956, Segmentation::BOTH, objects).await;
    let connection = connect(47957).await;
    let instance = scan::locate(&connection.client, None, Some(at(47956).into()), WAIT)
        .await
        .unwrap();

    let log = ObjectIdentifier::new(ObjectType::TREND_LOG_MULTIPLE, 1).unwrap();
    let members = trend::read_members(&connection.client, instance, log).await;
    let read = trend::read_trend_log(&connection.client, instance, log, 30).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let members = members.unwrap();
    assert_eq!(
        members
            .iter()
            .map(|member| member.object_identifier)
            .collect::<Vec<_>>(),
        vec![supply, fan]
    );
    let read = read.unwrap();
    assert_eq!(read.records.len(), 80);
    assert_eq!(read.records[0].values, vec![Some(20.0), Some(0.0)]);
    assert_eq!(read.records[79].values, vec![Some(27.9), Some(1.0)]);
}
