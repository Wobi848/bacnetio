mod common;

use std::time::Duration;

use bacnet_types::enums::{ObjectType, PropertyIdentifier, Segmentation};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use bacnetio::{connection, scan};
use common::{at, connect, simulated_device};

const WHO_IS_WAIT: Duration = Duration::from_millis(200);

#[tokio::test]
async fn finds_device_by_asking_directly_and_reads_its_details() {
    let mut device = simulated_device(1001, "AHU-01", 47910, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47911).await;

    let report = scan::run(
        &connection.client,
        WHO_IS_WAIT,
        vec![at(47910), at(47912)],
        &|_| {},
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(report.swept, 2);
    assert_eq!(report.devices.len(), 1);
    let found = &report.devices[0];
    assert_eq!(found.instance, 1001);
    assert_eq!(found.address, at(47910).into());
    assert!(!found.answered_who_is);
    assert_eq!(found.details.name.as_deref(), Some("AHU-01"));
    assert!(found.details.vendor.is_some());
    assert!(report.duplicates.is_empty());
}

#[tokio::test]
async fn reports_two_devices_with_the_same_id() {
    let mut first = simulated_device(7, "Boiler A", 47920, Segmentation::NONE, Vec::new()).await;
    let mut second = simulated_device(7, "Boiler B", 47921, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47922).await;

    let report = scan::run(
        &connection.client,
        WHO_IS_WAIT,
        vec![at(47920), at(47921)],
        &|_| {},
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    first.stop().await.unwrap();
    second.stop().await.unwrap();
    assert_eq!(report.devices.len(), 2);
    assert_eq!(report.duplicates[&7].len(), 2);
}

/// The client keeps the address it heard first for an ID: a device that took the ID of one gone
/// since must still be found by the next search (field test 3 November).
#[tokio::test]
async fn a_new_search_forgets_where_an_id_was() {
    let mut before = simulated_device(7, "Before", 47913, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47915).await;
    scan::run(&connection.client, WHO_IS_WAIT, vec![at(47913)], &|_| {})
        .await
        .unwrap();
    before.stop().await.unwrap();
    let mut after = simulated_device(7, "After", 47914, Segmentation::NONE, Vec::new()).await;

    let report = scan::run(&connection.client, WHO_IS_WAIT, vec![at(47914)], &|_| {})
        .await
        .unwrap();

    connection.close().await.unwrap();
    after.stop().await.unwrap();
    assert_eq!(report.devices.len(), 1);
    assert_eq!(report.devices[0].details.name.as_deref(), Some("After"));
}

/// Of two devices with one ID, requests by that ID reach the one aimed at.
#[tokio::test]
async fn requests_reach_the_device_aimed_at_among_two_with_one_id() {
    let mut first = simulated_device(7, "Boiler A", 47916, Segmentation::NONE, Vec::new()).await;
    let mut second = simulated_device(7, "Boiler B", 47917, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47918).await;
    let report = scan::run(
        &connection.client,
        WHO_IS_WAIT,
        vec![at(47916), at(47917)],
        &|_| {},
    )
    .await
    .unwrap();

    let mut names = Vec::new();
    for device in &report.devices {
        scan::aim_at(&connection.client, device).await.unwrap();
        let name = connection::read_value(
            &connection.client,
            7,
            ObjectIdentifier::new(ObjectType::DEVICE, 7).unwrap(),
            PropertyIdentifier::OBJECT_NAME,
            None,
        )
        .await
        .unwrap();
        names.push((device.address, name));
    }

    connection.close().await.unwrap();
    first.stop().await.unwrap();
    second.stop().await.unwrap();
    let name = |text: &str| PropertyValue::CharacterString(text.to_string());
    assert_eq!(
        names,
        vec![
            (at(47916).into(), name("Boiler A")),
            (at(47917).into(), name("Boiler B"))
        ]
    );
}

#[tokio::test]
async fn falls_back_to_another_port_when_the_port_is_taken() {
    let _taken = std::net::UdpSocket::bind(at(47930)).unwrap();

    let connection = connect(47930).await;

    assert!(connection.port_unavailable.is_some());
    assert_eq!(connection.local_port, 0);
    connection.close().await.unwrap();
}

/// I-Am-Router-To-Network (Clause 6.4.2) for `networks`, as a BACnet/IP unicast.
fn i_am_router(networks: &[u16]) -> Vec<u8> {
    let mut npdu = vec![0x01, 0x80, 0x01];
    for network in networks {
        npdu.extend(network.to_be_bytes());
    }
    let length = u16::try_from(4 + npdu.len()).unwrap();
    let mut packet = vec![0x81, 0x0A];
    packet.extend(length.to_be_bytes());
    packet.extend(npdu);
    packet
}

#[tokio::test]
async fn lists_routers_and_the_networks_behind_them() {
    let router = tokio::net::UdpSocket::bind(at(47925)).await.unwrap();
    let connection = connect(47926).await;
    let announce = async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        router
            .send_to(&i_am_router(&[5, 7]), at(47926))
            .await
            .unwrap();
    };

    let (report, ()) = tokio::join!(
        scan::run(&connection.client, WHO_IS_WAIT, Vec::new(), &|_| {}),
        announce
    );

    connection.close().await.unwrap();
    let report = report.unwrap();
    assert_eq!(report.routers.len(), 1);
    assert_eq!(report.routers[0].address, at(47925).into());
    assert_eq!(report.routers[0].networks, vec![5, 7]);
}

#[tokio::test]
async fn opens_one_device_by_its_address_with_details() {
    let mut device = simulated_device(1201, "Chiller", 47927, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47928).await;

    let found = scan::find_one(&connection.client, None, at(47927)).await;
    let missing = scan::find_one(&connection.client, None, at(47929)).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let found = found.unwrap();
    assert_eq!(found.instance, 1201);
    assert_eq!(found.details.name.as_deref(), Some("Chiller"));
    assert!(
        missing
            .unwrap_err()
            .to_string()
            .contains("no BACnet device answers")
    );
}

#[tokio::test]
async fn lists_only_devices_in_the_id_range() {
    let mut inside = simulated_device(1500, "Inside", 47932, Segmentation::NONE, Vec::new()).await;
    let mut outside =
        simulated_device(2500, "Outside", 47933, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47934).await;

    let report = scan::run_in_range(
        &connection.client,
        WHO_IS_WAIT,
        vec![at(47932), at(47933)],
        Some((1000, 1999)),
        false,
        &|_| {},
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    inside.stop().await.unwrap();
    outside.stop().await.unwrap();
    let found: Vec<u32> = report
        .devices
        .iter()
        .map(|device| device.instance)
        .collect();
    assert_eq!(found, vec![1500]);
}

/// Another BACnet program bound to the adapter's address on the port would get the devices'
/// answers; bacnetio takes another port and says why.
#[tokio::test]
async fn a_port_taken_on_the_adapter_address_is_avoided() {
    let other_program = std::net::UdpSocket::bind(at(47940)).unwrap();
    let connection = connect(47940).await;
    let reason = connection.port_unavailable.clone();
    connection.close().await.unwrap();
    drop(other_program);
    assert_eq!(
        reason.as_deref(),
        Some("another program uses 127.0.0.1:47940")
    );
}

/// A device shows while the scan still waits for silent addresses, not only at the end.
#[tokio::test]
async fn devices_show_while_the_scan_runs() {
    let mut device = simulated_device(1301, "Early", 47944, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47945).await;
    let seen = std::sync::Mutex::new(Vec::new());

    scan::run(
        &connection.client,
        WHO_IS_WAIT,
        vec![at(47944), at(47946)],
        &|progress| seen.lock().unwrap().push(progress),
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    let seen = seen.into_inner().unwrap();
    assert!(seen.iter().any(|progress| {
        matches!(progress.phase, scan::Phase::Sweep { asked: 1, total: 2 })
            && progress.devices.iter().any(|found| found.instance == 1301)
    }));
}

#[tokio::test]
async fn reads_every_object_list_during_the_search_when_asked() {
    let objects = || -> Vec<Box<dyn bacnet_objects::traits::BACnetObject>> {
        (1..=40)
            .map(|instance| {
                Box::new(
                    bacnet_objects::binary::BinaryValueObject::new(
                        instance,
                        format!("v{instance}"),
                    )
                    .unwrap(),
                ) as Box<dyn bacnet_objects::traits::BACnetObject>
            })
            .collect()
    };
    let mut devices = Vec::new();
    for offset in 0..6 {
        devices.push(
            simulated_device(
                100 + offset,
                "Controller",
                47960 + offset as u16,
                Segmentation::NONE,
                objects(),
            )
            .await,
        );
    }
    let connection = connect(47970).await;
    let targets = (0..6).map(|offset| at(47960 + offset)).collect();

    let report = scan::run_in_range(
        &connection.client,
        WHO_IS_WAIT,
        targets,
        None,
        true,
        &|_| {},
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    for device in &mut devices {
        device.stop().await.unwrap();
    }
    assert_eq!(report.devices.len(), 6);
    for device in &report.devices {
        // the 40 objects and the device object itself
        let list = device.objects.objects.as_ref().expect("list read");
        assert_eq!(list.len(), 41);
        assert_eq!(device.objects.total, Some(41));
        assert_eq!(device.objects.error, None);
    }
}

#[tokio::test]
async fn the_search_alone_reads_no_object_lists() {
    let mut device = simulated_device(7, "Boiler", 47972, Segmentation::NONE, Vec::new()).await;
    let connection = connect(47973).await;

    let report = scan::run(&connection.client, WHO_IS_WAIT, vec![at(47972)], &|_| {})
        .await
        .unwrap();

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(report.devices[0].objects, scan::ObjectList::default());
}

#[tokio::test]
async fn a_device_that_stays_silent_gets_a_reason_instead_of_a_list() {
    let connection = connect(47974).await;
    connection
        .client
        .add_device(999, &bacnetio::address::Address::from(at(47975)).mac())
        .await
        .unwrap();
    let (listed, _progress) = tokio::sync::mpsc::unbounded_channel();

    let list = scan::read_object_list(&connection.client, 999, &listed).await;

    connection.close().await.unwrap();
    assert!(list.error.is_some());
    assert_eq!(list.objects, None);
}
