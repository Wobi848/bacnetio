//! Active alarms of a device by GetEventInformation.

mod common;

use std::time::Duration;

use bacnet_types::enums::Segmentation;
use bacnetio::{alarms, scan};
use common::{at, connect, simulated_device};

#[tokio::test]
async fn a_device_without_alarms_lists_none() {
    let mut device = simulated_device(4001, "Quiet", 47948, Segmentation::BOTH, Vec::new()).await;
    let connection = connect(47949).await;
    let instance = scan::locate(
        &connection.client,
        None,
        Some(at(47948)),
        Duration::from_millis(500),
    )
    .await
    .unwrap();
    let found = alarms::read_alarms(&connection.client, instance).await;
    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(found.unwrap(), Vec::new());
}
