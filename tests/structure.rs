//! The plant view a device carries itself: Structured View objects with their subordinates.

mod common;

use std::time::Duration;

use bacnet_objects::analog::AnalogInputObject;
use bacnet_objects::group::StructuredViewObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::constructed::BACnetDeviceObjectReference;
use bacnet_types::enums::{EngineeringUnits, ObjectType, Segmentation};
use bacnet_types::primitives::ObjectIdentifier;
use bacnetio::{scan, structure};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

fn object(object_type: ObjectType, instance: u32) -> ObjectIdentifier {
    ObjectIdentifier::new(object_type, instance).unwrap()
}

/// Two sensors, a view that holds them and a root view that holds that view and an object on
/// another device, as a controller's system view does.
fn plant_objects() -> Vec<Box<dyn BACnetObject>> {
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let supply = AnalogInputObject::new(1, "Supply air temp", celsius).unwrap();
    let outdoor = AnalogInputObject::new(2, "Outdoor air temp", celsius).unwrap();
    let mut air = StructuredViewObject::new(2, "Air handling").unwrap();
    air.add_subordinate(object(ObjectType::ANALOG_INPUT, 1), "Supply air")
        .unwrap();
    air.add_subordinate(object(ObjectType::ANALOG_INPUT, 2), "Outdoor air")
        .unwrap();
    let mut root = StructuredViewObject::new(1, "Plant").unwrap();
    root.add_subordinate(object(ObjectType::STRUCTURED_VIEW, 2), "Air handling")
        .unwrap();
    root.add_subordinate(
        BACnetDeviceObjectReference {
            device_identifier: Some(object(ObjectType::DEVICE, 1002)),
            object_identifier: object(ObjectType::ANALOG_INPUT, 7),
        },
        "Weather station",
    )
    .unwrap();
    vec![
        Box::new(supply),
        Box::new(outdoor),
        Box::new(root),
        Box::new(air),
    ]
}

#[tokio::test]
async fn reads_the_views_of_a_device_with_their_subordinates_and_annotations() {
    let mut device =
        simulated_device(1001, "AHU-01", 47990, Segmentation::BOTH, plant_objects()).await;
    let connection = connect(47991).await;

    let instance = scan::locate(&connection.client, Some(1001), Some(at(47990)), WAIT)
        .await
        .unwrap();
    let listed = bacnetio::object_list::read(
        &connection.client,
        instance,
        bacnetio::limits::Limits::ASSUMED,
        &|_| {},
    )
    .await
    .unwrap()
    .0;
    let views = structure::views_of(listed);
    let read = structure::read_views(&connection.client, instance, &views).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(views.len(), 2, "{views:?}");
    let root = read
        .iter()
        .find(|view| view.object == object(ObjectType::STRUCTURED_VIEW, 1))
        .expect("the root view");
    assert_eq!(root.error, None);
    assert_eq!(root.subordinates.len(), 2);
    assert_eq!(
        root.subordinates[0].object,
        object(ObjectType::STRUCTURED_VIEW, 2)
    );
    assert_eq!(root.subordinates[0].device, None);
    assert_eq!(
        root.subordinates[0].annotation.as_deref(),
        Some("Air handling")
    );
    // an object on another device keeps that device, so the tree can say where it is
    assert_eq!(root.subordinates[1].device, Some(1002));
    assert_eq!(
        root.subordinates[1].object,
        object(ObjectType::ANALOG_INPUT, 7)
    );
    let air = read
        .iter()
        .find(|view| view.object == object(ObjectType::STRUCTURED_VIEW, 2))
        .expect("the air handling view");
    assert_eq!(
        air.subordinates
            .iter()
            .map(|entry| entry.object.instance_number())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        air.subordinates[1].annotation.as_deref(),
        Some("Outdoor air")
    );
}

#[tokio::test]
async fn a_device_without_structured_views_has_no_plant_view() {
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let sensor = AnalogInputObject::new(1, "Supply air temp", celsius).unwrap();
    let mut device = simulated_device(
        1003,
        "No view",
        47992,
        Segmentation::BOTH,
        vec![Box::new(sensor)],
    )
    .await;
    let connection = connect(47993).await;

    let instance = scan::locate(&connection.client, Some(1003), Some(at(47992)), WAIT)
        .await
        .unwrap();
    let listed = bacnetio::object_list::read(
        &connection.client,
        instance,
        bacnetio::limits::Limits::ASSUMED,
        &|_| {},
    )
    .await
    .unwrap()
    .0;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(structure::views_of(listed).is_empty());
}
