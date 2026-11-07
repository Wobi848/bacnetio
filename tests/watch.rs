mod common;

use std::time::{Duration, Instant, SystemTime};

use bacnet_objects::analog::AnalogOutputObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::enums::{EngineeringUnits, ObjectType, Segmentation};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use bacnetio::scan;
use bacnetio::watch::{self, Watch};
use bacnetio::write::{self, Action};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

#[tokio::test]
async fn the_device_reports_changes_after_a_subscription() {
    let valve = AnalogOutputObject::new(1, "Heating valve", EngineeringUnits::PERCENT).unwrap();
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(valve)];
    let mut device = simulated_device(6001, "AHU-06", 47930, Segmentation::BOTH, objects).await;
    let connection = connect(47931).await;
    let client = &connection.client;
    let instance = scan::locate(client, None, Some(at(47930).into()), WAIT)
        .await
        .unwrap();
    let object = ObjectIdentifier::new(ObjectType::ANALOG_OUTPUT, 1).unwrap();
    let mut notifications = client.cov_notifications();
    let mut watch = Watch::default();

    watch::subscribe(client, instance, object).await.unwrap();
    watch.subscribed(instance, object, Instant::now());
    write::perform(
        client,
        instance,
        object,
        Some(8),
        &Action::Write(PropertyValue::Real(42.0)),
    )
    .await
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let Ok(Ok(received)) =
            tokio::time::timeout(Duration::from_millis(200), notifications.recv()).await
        else {
            continue;
        };
        watch.apply(&received.notification, SystemTime::now());
        if watch
            .live(instance, object, Instant::now())
            .is_some_and(|live| live.value == PropertyValue::Real(42.0))
        {
            break;
        }
    }

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let live = watch.live(instance, object, Instant::now()).cloned();
    assert_eq!(live.map(|live| live.value), Some(PropertyValue::Real(42.0)));
}
