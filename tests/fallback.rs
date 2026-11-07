//! A device that does not know ReadPropertyMultiple, as old controllers: a proxy in front of a
//! simulated device rejects every ReadPropertyMultiple and passes everything else on.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bacnet_objects::analog::AnalogInputObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::enums::{EngineeringUnits, Segmentation};
use bacnetio::points::{self, ReadNote};
use bacnetio::scan;
use common::{at, connect, simulated_device};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

const WAIT: Duration = Duration::from_millis(500);
const CONFIRMED_REQUEST: u8 = 0x00;
const READ_PROPERTY_MULTIPLE: u8 = 14;
const UNRECOGNIZED_SERVICE: u8 = 9;

/// Invoke ID of a confirmed ReadPropertyMultiple request in a BACnet/IP packet without routing.
fn read_property_multiple(packet: &[u8]) -> Option<u8> {
    let apdu = packet.get(6..)?;
    let is_simple_npdu = packet.get(5) == Some(&0x04) || packet.get(5) == Some(&0x00);
    (is_simple_npdu
        && apdu.first()? & 0xF0 == CONFIRMED_REQUEST
        && *apdu.get(3)? == READ_PROPERTY_MULTIPLE)
        .then(|| apdu[2])
}

fn reject(invoke_id: u8) -> Vec<u8> {
    vec![
        0x81,
        0x0A,
        0x00,
        0x09,
        0x01,
        0x00,
        0x60,
        invoke_id,
        UNRECOGNIZED_SERVICE,
    ]
}

async fn proxy(listen: u16, device: u16) {
    let front = Arc::new(UdpSocket::bind(at(listen)).await.unwrap());
    let back = Arc::new(UdpSocket::bind(at(0)).await.unwrap());
    let client: Arc<Mutex<Option<SocketAddr>>> = Arc::default();
    let (to_client, remembered) = (Arc::clone(&front), Arc::clone(&client));
    let from_device = Arc::clone(&back);
    tokio::spawn(async move {
        let mut buffer = [0u8; 1500];
        loop {
            let (length, _) = from_device.recv_from(&mut buffer).await.unwrap();
            if let Some(client) = *remembered.lock().await {
                to_client.send_to(&buffer[..length], client).await.unwrap();
            }
        }
    });
    tokio::spawn(async move {
        let mut buffer = [0u8; 1500];
        loop {
            let (length, sender) = front.recv_from(&mut buffer).await.unwrap();
            *client.lock().await = Some(sender);
            let packet = &buffer[..length];
            match read_property_multiple(packet) {
                Some(invoke_id) => front.send_to(&reject(invoke_id), sender).await.unwrap(),
                None => back.send_to(packet, at(device)).await.unwrap(),
            };
        }
    });
}

#[tokio::test]
async fn reads_one_by_one_when_read_property_multiple_is_unknown() {
    let objects: Vec<Box<dyn BACnetObject>> = (0..5)
        .map(|instance| {
            let mut sensor = AnalogInputObject::new(
                instance,
                format!("Sensor {instance}"),
                EngineeringUnits::PERCENT,
            )
            .unwrap();
            sensor.set_present_value(instance as f32);
            Box::new(sensor) as Box<dyn BACnetObject>
        })
        .collect();
    let mut device = simulated_device(5001, "Old DDC", 47997, Segmentation::BOTH, objects).await;
    proxy(47998, 47997).await;
    let connection = connect(47999).await;

    let instance = scan::locate(&connection.client, None, Some(at(47998).into()), WAIT)
        .await
        .unwrap();
    let reading = points::read_device(&connection.client, instance)
        .await
        .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(
        reading.notes.contains(&ReadNote::NoReadPropertyMultiple),
        "{:?}",
        reading.notes
    );
    let sensors: Vec<_> = reading
        .points
        .iter()
        .filter(|point| {
            point
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with("Sensor"))
        })
        .collect();
    assert_eq!(sensors.len(), 5);
    assert!(
        sensors
            .iter()
            .all(|point| point.error.is_none() && point.value.is_some())
    );
}
