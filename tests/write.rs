mod common;

use std::time::{Duration, SystemTime};

use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject};
use bacnet_objects::traits::BACnetObject;
use bacnet_types::enums::{EngineeringUnits, ObjectType, Segmentation};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use bacnetio::object;
use bacnetio::scan;
use bacnetio::write::{self, Action, Entry, Journal};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn objects() -> Vec<Box<dyn BACnetObject>> {
    let mut valve = AnalogOutputObject::new(1, "Heating valve", EngineeringUnits::PERCENT).unwrap();
    valve.set_relinquish_default(10.0).unwrap();
    let sensor = AnalogInputObject::new(2, "Room temp", EngineeringUnits::DEGREES_CELSIUS).unwrap();
    vec![Box::new(valve), Box::new(sensor)]
}

fn analog(object_type: ObjectType, instance: u32) -> ObjectIdentifier {
    ObjectIdentifier::new(object_type, instance).unwrap()
}

#[tokio::test]
async fn writes_at_a_priority_and_releases_again() {
    let mut device = simulated_device(3001, "HC-01", 47990, Segmentation::BOTH, objects()).await;
    let connection = connect(47991).await;
    let client = &connection.client;
    let instance = scan::locate(client, None, Some(at(47990).into()), WAIT)
        .await
        .unwrap();
    let valve = analog(ObjectType::ANALOG_OUTPUT, 1);

    let before = object::read_object(client, instance, valve).await;
    let value = write::parse_value(ObjectType::ANALOG_OUTPUT, "42,5").unwrap();
    let mut journal = Journal::default();
    for (priority, action) in [(8, Action::Write(value)), (8, Action::Release)] {
        let result = write::perform(client, instance, valve, Some(priority), &action).await;
        let during = object::read_object(client, instance, valve).await;
        journal.record(Entry {
            time: SystemTime::now(),
            device: instance,
            address: Some(at(47990).into()),
            object: valve,
            priority: Some(priority),
            action: action.clone(),
            error: result.err().map(|error| error.to_string()),
        });
        if matches!(action, Action::Write(_)) {
            assert_eq!(during.point.value, Some(PropertyValue::Real(42.5)));
            assert_eq!(
                during.priority_array.as_ref().unwrap()[7],
                Some(PropertyValue::Real(42.5))
            );
            assert_eq!(journal.held().len(), 1);
        }
    }
    let after = object::read_object(client, instance, valve).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(before.commandable());
    assert_eq!(before.relinquish_default, Some(PropertyValue::Real(10.0)));
    assert!(before.priority_array.unwrap().iter().all(Option::is_none));
    assert_eq!(after.point.value, Some(PropertyValue::Real(10.0)));
    assert!(journal.held().is_empty());
    assert!(journal.entries().iter().all(|entry| entry.error.is_none()));
}

#[tokio::test]
async fn an_input_is_not_commandable_and_refuses_writes() {
    let mut device = simulated_device(3002, "HC-02", 47992, Segmentation::BOTH, objects()).await;
    let connection = connect(47993).await;
    let client = &connection.client;
    let instance = scan::locate(client, None, Some(at(47992).into()), WAIT)
        .await
        .unwrap();
    let sensor = analog(ObjectType::ANALOG_INPUT, 2);

    let detail = object::read_object(client, instance, sensor).await;
    let refused = write::perform(
        client,
        instance,
        sensor,
        None,
        &Action::Write(PropertyValue::Real(5.0)),
    )
    .await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(!detail.commandable());
    assert_eq!(detail.point.name.as_deref(), Some("Room temp"));
    assert_eq!(detail.point.error, None);
    assert!(refused.is_err());
}
