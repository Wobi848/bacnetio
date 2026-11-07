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
    // right above the highest station: polled first, so taken into the ring at once
    assert_eq!(heard.proposed_station, Some(10));
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

/// A device as station 5 on the other end of an in-memory trunk.
async fn device_on(
    serial: bacnet_transport::mstp::LoopbackSerial,
) -> bacnet_server::server::BACnetServer<
    bacnet_transport::mstp::MstpTransport<bacnet_transport::mstp::LoopbackSerial>,
> {
    use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject};
    use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
    use bacnet_objects::traits::BACnetObject;
    use bacnet_transport::mstp::{MstpConfig, MstpTransport};
    use bacnet_types::enums::EngineeringUnits;
    let mut temperature =
        AnalogInputObject::new(1, "Room temperature", EngineeringUnits::DEGREES_CELSIUS).unwrap();
    temperature.set_present_value(21.5);
    let mut damper = AnalogOutputObject::new(1, "Damper", EngineeringUnits::PERCENT).unwrap();
    damper.set_relinquish_default(0.0).unwrap();
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(temperature), Box::new(damper)];
    let identity = DeviceIdentity::new(2005, 42)
        .unwrap()
        .with_max_apdu(480)
        .unwrap()
        .with_name("MSTP-05");
    let database = build_database_with_extra(&identity, objects).unwrap();
    let config = MstpConfig {
        this_station: 5,
        baud_rate: 38_400,
        ..MstpConfig::default()
    };
    bacnet_server::server::BACnetServer::generic_builder()
        .transport(MstpTransport::new(serial, config))
        .device(identity)
        .database(database)
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn joining_a_trunk_finds_its_device_and_reads_it() {
    use bacnetio::address::Address;
    use bacnetio::connection::{Trunk, open_mstp_on};
    let (ours, theirs) = bacnet_transport::mstp::LoopbackSerial::pair();
    let _device = device_on(theirs).await;
    let trunk = Trunk {
        port: "loopback".to_string(),
        baud: 38_400,
        station: 6,
        max_master: 127,
    };
    let connection = open_mstp_on(ours, &trunk).await.unwrap();
    let report = bacnetio::scan::run(
        &connection.client,
        Duration::from_secs(3),
        Vec::new(),
        &|_| {},
    )
    .await
    .unwrap();
    let found: Vec<(u32, Address)> = report
        .devices
        .iter()
        .map(|device| (device.instance, device.address))
        .collect();
    assert_eq!(found, [(2005, Address::Mstp(5))]);
    assert_eq!(report.devices[0].details.name.as_deref(), Some("MSTP-05"));
    let reading = bacnetio::points::read_device(&connection.client, 2005)
        .await
        .unwrap();
    let temperature = reading
        .points
        .iter()
        .find(|point| point.name.as_deref() == Some("Room temperature"))
        .expect("the point is read");
    assert!(
        format!("{:?}", temperature.value).contains("21.5"),
        "{:?}",
        temperature.value
    );
    connection.close().await.unwrap();
}

/// bacnetio on an in-memory trunk with the device as station 5, the device known to the client.
async fn on_trunk() -> (
    bacnet_server::server::BACnetServer<
        bacnet_transport::mstp::MstpTransport<bacnet_transport::mstp::LoopbackSerial>,
    >,
    bacnetio::connection::Connection,
) {
    use bacnetio::connection::{Trunk, open_mstp_on};
    let (ours, theirs) = bacnet_transport::mstp::LoopbackSerial::pair();
    let device = device_on(theirs).await;
    let trunk = Trunk {
        port: "loopback".to_string(),
        baud: 38_400,
        station: 6,
        max_master: 127,
    };
    let connection = open_mstp_on(ours, &trunk).await.unwrap();
    let station = bacnetio::address::Address::Mstp(5);
    bacnetio::scan::locate(
        &connection.client,
        Some(2005),
        Some(station),
        Duration::from_secs(3),
    )
    .await
    .unwrap();
    (device, connection)
}

#[tokio::test]
async fn writing_releasing_and_cov_reports_work_over_the_trunk() {
    use bacnet_types::enums::ObjectType;
    use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
    use bacnetio::write::{self, Action};
    let (_device, connection) = on_trunk().await;
    let client = &connection.client;
    let damper = ObjectIdentifier::new(ObjectType::ANALOG_OUTPUT, 1).unwrap();
    let mut notifications = client.cov_notifications();
    bacnetio::watch::subscribe(client, 2005, damper)
        .await
        .unwrap();
    write::perform(
        client,
        2005,
        damper,
        Some(8),
        &Action::Write(PropertyValue::Real(42.0)),
    )
    .await
    .unwrap();
    let held = bacnetio::object::read_object(client, 2005, damper).await;
    assert_eq!(held.point.value, Some(PropertyValue::Real(42.0)));
    assert_eq!(
        held.priority_array.unwrap()[7],
        Some(PropertyValue::Real(42.0))
    );
    // the device reports the change by itself, over the trunk
    let reported = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let received = notifications.recv().await.unwrap();
            let value = received
                .notification
                .list_of_values
                .iter()
                .find_map(|value| {
                    (value.property_identifier
                        == bacnet_types::enums::PropertyIdentifier::PRESENT_VALUE)
                        .then(|| value.value.clone())
                });
            let decoded = value.and_then(|value| bacnetio::connection::decode_value(&value).ok());
            if decoded == Some(PropertyValue::Real(42.0)) {
                return;
            }
        }
    })
    .await;
    assert!(reported.is_ok(), "no COV report of 42 within 5 s");
    write::perform(client, 2005, damper, Some(8), &Action::Release)
        .await
        .unwrap();
    let released = bacnetio::object::read_object(client, 2005, damper).await;
    assert_eq!(released.point.value, Some(PropertyValue::Real(0.0)));
    connection.close().await.unwrap();
}
