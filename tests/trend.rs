mod common;

use std::time::Duration;

use bacnet_objects::traits::BACnetObject;
use bacnet_objects::trend::TrendLogObject;
use bacnet_types::constructed::{BACnetLogRecord, LogDatum};
use bacnet_types::enums::{ObjectType, Segmentation};
use bacnet_types::primitives::{Date, ObjectIdentifier, Time};
use bacnetio::{scan, trend};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn supply_air_log(records: u32) -> TrendLogObject {
    let mut log = TrendLogObject::new(1, "Supply air temp log", 1000).unwrap();
    for minute in 0..records {
        log.add_record(BACnetLogRecord {
            date: Date {
                year: 126,
                month: 11,
                day: 1,
                day_of_week: 7,
            },
            time: Time {
                hour: (8 + minute / 60) as u8,
                minute: (minute % 60) as u8,
                second: 0,
                hundredths: 0,
            },
            log_datum: LogDatum::RealValue(20.0 + minute as f32 / 10.0),
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
    assert_eq!(read.records[0].value, Some(20.0));
    assert_eq!(read.records[119].value, Some(31.9));
    assert_eq!(
        (read.records[119].time.hour, read.records[119].time.minute),
        (9, 59)
    );
}
