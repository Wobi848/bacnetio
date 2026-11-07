//! Listening to an MS/TP trunk (docs/concept-mstp.md, step 2) over an in-memory line: the speed is
//! found, the stations and a free address are named, and nothing is ever sent.
#![cfg(not(target_os = "android"))]

use std::time::Duration;

use bacnet_transport::mstp::codec::{Frame, FrameType};
use bacnet_transport::mstp::monitor::DropOnClose;
use bacnetio::mstp::listen;
use tokio::io::{AsyncWriteExt, DuplexStream, ReadHalf};

const TRUNK_SPEED: u32 = 76_800;
const PER_SPEED: Duration = Duration::from_millis(300);

fn frame(frame_type: FrameType, destination: u8, source: u8) -> Vec<u8> {
    let mut wire = [0u8; 8];
    let length = Frame::new(frame_type, destination, source, &[])
        .encode_into(&mut wire)
        .unwrap();
    wire[..length].to_vec()
}

/// Stations 5 and 9 pass the token between them; 9 polls up to its Max_Master of 63, then wraps.
fn ring() -> Vec<u8> {
    let mut wire = Vec::new();
    for _ in 0..5 {
        wire.extend(frame(FrameType::TOKEN, 9, 5));
        wire.extend(frame(FrameType::TOKEN, 5, 9));
    }
    for polled in [10, 63, 0] {
        wire.extend(frame(FrameType::POLL_FOR_MASTER, polled, 9));
    }
    wire
}

/// A line at `baud`: the ring at the trunk's speed, silence at any other.
fn line(baud: u32) -> anyhow::Result<DropOnClose<ReadHalf<DuplexStream>>> {
    let (mut trunk, ours) = tokio::io::duplex(4096);
    let (reader, writer) = tokio::io::split(ours);
    tokio::spawn(async move {
        // ours to keep the line open; nothing is ever written to it
        let _not_written = writer;
        if baud == TRUNK_SPEED {
            trunk.write_all(&ring()).await.unwrap();
        }
        tokio::time::sleep(PER_SPEED * 2).await;
    });
    Ok(DropOnClose::new(reader))
}

#[tokio::test]
async fn listening_finds_the_speed_the_stations_and_a_free_address() {
    let heard = listen(line, &bacnetio::mstp::SPEEDS, PER_SPEED)
        .await
        .unwrap();
    assert_eq!(heard.baud, Some(TRUNK_SPEED));
    // 38400 was tried first and heard nothing; the search stopped at the trunk's speed
    let tried: Vec<u32> = heard.trials.iter().map(|trial| trial.baud).collect();
    assert_eq!(tried, [38_400, 76_800]);
    assert_eq!(heard.trials[0].frames, 0);
    assert_eq!(heard.stations, [5, 9]);
    assert_eq!(heard.max_master, Some(63));
    // 63 is polled but silent: free, and still inside the ring's Max_Master
    assert_eq!(heard.proposed_station, Some(63));
}

#[tokio::test]
async fn a_silent_line_finds_no_speed_and_proposes_nothing() {
    let silent = |_baud: u32| line(0);
    let heard = listen(silent, &[38_400, 9_600], PER_SPEED).await.unwrap();
    assert_eq!(heard.baud, None);
    assert_eq!(heard.trials.len(), 2);
    assert!(heard.stations.is_empty());
    assert_eq!(heard.proposed_station, None);
}
