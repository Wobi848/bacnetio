mod common;

use std::time::Duration;

use bacnet_objects::analog::AnalogInputObject;
use bacnet_objects::binary::BinaryValueObject;
use bacnet_objects::multistate::MultiStateValueObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::enums::PropertyIdentifier;
use bacnet_types::enums::{EngineeringUnits, ObjectType, Segmentation};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use bacnetio::points::ReadNote;
use bacnetio::precedence::Precedence;
use bacnetio::{object, observe, points, present, property_text, scan};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn plant_objects() -> Vec<Box<dyn BACnetObject>> {
    let mut supply =
        AnalogInputObject::new(1, "Supply air temp", EngineeringUnits::DEGREES_CELSIUS).unwrap();
    supply.set_present_value(21.4);
    supply.set_description("AHU-01 supply");
    let mut pump = BinaryValueObject::new(2, "Pump on").unwrap();
    pump.set_relinquish_default(1).unwrap();
    let mut mode = MultiStateValueObject::new(3, "Operating mode", 4).unwrap();
    mode.write_property(
        PropertyIdentifier::STATE_TEXT,
        Some(1),
        PropertyValue::CharacterString("Off".into()),
        None,
    )
    .unwrap();
    vec![Box::new(supply), Box::new(pump), Box::new(mode)]
}

fn analog_inputs(count: u32) -> Vec<Box<dyn BACnetObject>> {
    (0..count)
        .map(|instance| {
            let object: Box<dyn BACnetObject> = Box::new(
                AnalogInputObject::new(
                    instance,
                    format!("Sensor {instance}"),
                    EngineeringUnits::PERCENT,
                )
                .unwrap(),
            );
            object
        })
        .collect()
}

#[tokio::test]
async fn reads_names_values_and_units_of_a_device_found_by_address() {
    let mut device =
        simulated_device(1001, "AHU-01", 47940, Segmentation::BOTH, plant_objects()).await;
    let connection = connect(47941).await;

    let instance = scan::locate(&connection.client, None, Some(at(47940)), WAIT)
        .await
        .unwrap();
    let reading = points::read_device(&connection.client, instance)
        .await
        .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(instance, 1001);
    assert!(reading.notes.is_empty(), "{:?}", reading.notes);
    let find = |object_type| {
        reading
            .points
            .iter()
            .find(|point| point.object.object_type() == object_type)
            .unwrap()
    };
    let supply = find(ObjectType::ANALOG_INPUT);
    assert_eq!(supply.name.as_deref(), Some("Supply air temp"));
    assert_eq!(supply.description.as_deref(), Some("AHU-01 supply"));
    assert_eq!(present::value_text(supply), "21.4 °C");
    assert_eq!(present::status_text(supply.status), "");
    assert_eq!(
        present::value_text(find(ObjectType::BINARY_VALUE)),
        "active"
    );
    let mode = find(ObjectType::MULTI_STATE_VALUE);
    assert_eq!(present::value_text(mode), "state 1");
    assert_eq!(mode.state_texts, ["Off", "State 2", "State 3", "State 4"]);
    assert_eq!(present::shown_value(mode), "Off");
    assert!(find(ObjectType::DEVICE).error.is_none());
}

#[tokio::test]
async fn reads_a_large_object_list_entry_by_entry_without_segmentation() {
    let mut device = simulated_device(
        2040,
        "Boiler room",
        47950,
        Segmentation::NONE,
        analog_inputs(400),
    )
    .await;
    let connection = connect(47951).await;

    let instance = scan::locate(&connection.client, Some(2040), Some(at(47950)), WAIT)
        .await
        .unwrap();
    let reading = points::read_device(&connection.client, instance)
        .await
        .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(reading.points.len(), 401);
    assert!(reading.notes.contains(&ReadNote::ObjectListEntryByEntry));
    assert!(reading.points.iter().all(|point| point.error.is_none()));
    assert!(reading.points.iter().all(|point| point.name.is_some()));
}

#[tokio::test]
async fn refuses_a_device_with_another_id_at_the_address() {
    let mut device = simulated_device(5, "VAV-5", 47960, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47961).await;

    let result = scan::locate(&connection.client, Some(6), Some(at(47960)), WAIT).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(result.unwrap_err().to_string().contains("has ID 5, not 6"));
}

#[tokio::test]
async fn names_the_way_out_when_a_device_does_not_answer_who_is() {
    let connection = connect(47971).await;

    let result = scan::locate(&connection.client, Some(77), None, WAIT).await;

    connection.close().await.unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("answer only when asked at its address")
    );
}

#[tokio::test]
async fn reads_every_property_of_an_object() {
    let mut device =
        simulated_device(1003, "AHU-03", 47942, Segmentation::BOTH, plant_objects()).await;
    let connection = connect(47943).await;
    let instance = scan::locate(&connection.client, None, Some(at(47942)), WAIT)
        .await
        .unwrap();

    let supply = ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 1).unwrap();
    let readings = object::read_properties(&connection.client, instance, supply).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let value = |property| {
        readings
            .iter()
            .find(|reading| reading.property == property)
            .map(|reading| reading.values.clone())
    };
    assert!(readings.len() > 8, "{readings:?}");
    assert_eq!(
        value(PropertyIdentifier::OBJECT_NAME),
        Some(Ok(vec![PropertyValue::CharacterString(
            "Supply air temp".into()
        )]))
    );
    let units = value(PropertyIdentifier::UNITS).unwrap().unwrap();
    assert_eq!(
        property_text::property_text(
            ObjectType::ANALOG_INPUT,
            PropertyIdentifier::UNITS,
            &units,
            None,
            &[]
        ),
        "°C"
    );
}

/// A device without segmentation is read with requests planned from its limits: not one request
/// fails, the object list comes in a few parts, and no answer is larger than the device sends.
#[tokio::test]
async fn reads_a_device_without_segmentation_without_a_failed_request() {
    let mut device = simulated_device(
        2050,
        "Plant room",
        47954,
        Segmentation::NONE,
        analog_inputs(400),
    )
    .await;
    let connection = connect(47955).await;
    let instance = scan::locate(&connection.client, Some(2050), Some(at(47954)), WAIT)
        .await
        .unwrap();
    let requests = std::sync::Mutex::new(Vec::new());
    let reading = points::read_device_observed(
        &connection.client,
        instance,
        None,
        &|event| {
            if let observe::Event::Request(request) = event {
                requests.lock().unwrap().push(request);
            }
        },
        &Precedence::default(),
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let requests = requests.into_inner().unwrap();
    let failed: Vec<_> = requests
        .iter()
        .filter(|request| request.failed.is_some())
        .collect();
    assert!(failed.is_empty(), "{failed:?}");
    let list_parts = requests
        .iter()
        .filter(|request| request.what.contains("object list"))
        .count();
    // the simulator takes 1024 bytes: about 80 entries per request instead of 400 requests
    assert!(list_parts <= 6, "{list_parts} requests for the object list");
    assert!(
        requests
            .iter()
            .all(|request| !request.what.contains("16 objects"))
    );
    assert_eq!(reading.points.len(), 401);
    assert!(reading.points.iter().all(|point| point.error.is_none()));
}

/// The reading of a list waits while a request a user waits for runs, and goes on after it.
#[tokio::test]
async fn reading_a_list_gives_way_to_a_user_request() {
    let mut device =
        simulated_device(1004, "AHU-04", 47937, Segmentation::BOTH, plant_objects()).await;
    let connection = connect(47938).await;
    let instance = scan::locate(&connection.client, Some(1004), Some(at(47937)), WAIT)
        .await
        .unwrap();
    let precedence = Precedence::default();
    let first = precedence.first();
    let read = std::sync::atomic::AtomicUsize::new(0);
    let observer = |event: observe::Event<'_>| {
        if let observe::Event::Values { read: count, .. } = event {
            read.store(count, std::sync::atomic::Ordering::SeqCst);
        }
    };
    let reading = {
        let reading = points::read_device_observed(
            &connection.client,
            instance,
            None,
            &observer,
            &precedence,
        );
        tokio::pin!(reading);
        let waited = tokio::time::timeout(Duration::from_millis(500), reading.as_mut()).await;
        assert!(waited.is_err(), "the reading waits");
        assert_eq!(read.load(std::sync::atomic::Ordering::SeqCst), 0);
        drop(first);
        reading.await.unwrap()
    };

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(reading.points.iter().all(|point| point.error.is_none()));
    assert_eq!(
        read.load(std::sync::atomic::Ordering::SeqCst),
        reading.points.len()
    );
}
