mod common;

use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use bacnet_objects::analog::AnalogInputObject;
use bacnet_objects::binary::BinaryValueObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::enums::{EngineeringUnits, ObjectType, Segmentation};
use bacnet_types::primitives::ObjectIdentifier;
use bacnetio::record::{self, RecordedPoint, Recording};
use bacnetio::watch::{self, Watch};
use bacnetio::{export, scan};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn recorded(object_type: ObjectType, instance: u32) -> RecordedPoint {
    RecordedPoint {
        device: 4001,
        object: ObjectIdentifier::new(object_type, instance).unwrap(),
        name: None,
        units: None,
    }
}

#[tokio::test]
async fn samples_numbers_and_gaps_into_the_csv() {
    let mut supply =
        AnalogInputObject::new(1, "Supply air temp", EngineeringUnits::DEGREES_CELSIUS).unwrap();
    supply.set_present_value(21.5);
    let pump = BinaryValueObject::new(2, "Pump").unwrap();
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(supply), Box::new(pump)];
    let mut device = simulated_device(4001, "AHU-04", 47995, Segmentation::BOTH, objects).await;
    let connection = connect(47996).await;
    scan::locate(&connection.client, None, Some(at(47995)), WAIT)
        .await
        .unwrap();

    let points = vec![
        recorded(ObjectType::ANALOG_INPUT, 1),
        recorded(ObjectType::BINARY_VALUE, 2),
        recorded(ObjectType::ANALOG_INPUT, 99),
    ];
    let watch = Mutex::new(Watch::default());
    let mut values = Vec::new();
    for point in &points {
        let value = watch::current_value(&watch, &connection.client, point.device, point.object)
            .await
            .ok()
            .and_then(|(live, _)| record::number(&live.value));
        values.push(value);
    }

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(values, vec![Some(21.5), Some(0.0), None]);

    let mut recording = Recording::default();
    for point in &points {
        recording.add(point.clone()).unwrap();
    }
    let round: Vec<_> = points.into_iter().zip(values).collect();
    recording.record(SystemTime::UNIX_EPOCH, &round);
    let csv = export::recording_csv(&recording, 0);
    assert!(csv.contains("1970-01-01 00:00:00;21.5;0;\r\n"), "{csv}");
}
