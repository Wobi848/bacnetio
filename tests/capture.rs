//! Capturing bacnetio's own traffic for Wireshark (docs/concept-toolbox.md part 3), against a
//! simulated device on the loopback interface.

mod common;

use std::sync::Arc;
use std::time::Duration;

use bacnet_types::enums::Segmentation;
use bacnetio::capture::Capture;
use bacnetio::{points, scan};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);
const DEVICE_PORT: u16 = 48010;
const CLIENT_PORT: u16 = 48011;

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// The data of each packet block of a pcapng file.
fn packets(file: &[u8]) -> Vec<&[u8]> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < file.len() {
        let length = u32_at(file, at + 4) as usize;
        if u32_at(file, at) == 6 {
            let captured = u32_at(file, at + 20) as usize;
            found.push(&file[at + 28..at + 28 + captured]);
        }
        at += length;
    }
    found
}

#[tokio::test]
async fn the_traffic_of_a_reading_is_captured_both_ways_and_nothing_after_the_stop() {
    let mut device =
        simulated_device(3101, "AHU-07", DEVICE_PORT, Segmentation::BOTH, Vec::new()).await;
    let connection = connect(CLIENT_PORT).await;
    let capture = Arc::new(Capture::default());
    assert_eq!(capture.state().frames, 0);

    capture.start(Some(&connection.client));
    let instance = scan::locate(&connection.client, None, Some(at(DEVICE_PORT).into()), WAIT)
        .await
        .unwrap();
    points::read_device(&connection.client, instance)
        .await
        .unwrap();
    // the tap hands frames over through a queue
    tokio::time::sleep(Duration::from_millis(200)).await;
    capture.stop();
    let captured = capture.state().frames;
    points::read_device(&connection.client, instance)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let after_stop = capture.state();
    let file = capture.pcapng();
    capture.clear();
    let cleared = capture.state().frames;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(captured >= 4, "requests and answers: {captured}");
    assert!(!after_stop.running);
    assert_eq!(after_stop.frames, captured, "nothing after the stop");
    assert_eq!(cleared, 0);
    let packets = packets(&file);
    assert_eq!(packets.len(), captured);
    let ports = |packet: &[u8]| {
        (
            u16::from_be_bytes([packet[20], packet[21]]),
            u16::from_be_bytes([packet[22], packet[23]]),
        )
    };
    assert!(
        packets
            .iter()
            .all(|packet| packet[0] == 0x45 && packet[28] == 0x81),
        "IPv4 with a BVLL message"
    );
    assert!(
        packets
            .iter()
            .any(|packet| ports(packet) == (CLIENT_PORT, DEVICE_PORT)),
        "sent"
    );
    assert!(
        packets
            .iter()
            .any(|packet| ports(packet) == (DEVICE_PORT, CLIENT_PORT)),
        "received"
    );
}
