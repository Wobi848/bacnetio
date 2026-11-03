mod common;

use std::time::Duration;

use bacnet_types::enums::Segmentation;
use bacnetio::{bbmd, scan};
use common::{at, connect, connect_via, simulated_bbmd, simulated_device};

#[tokio::test]
async fn finds_devices_through_a_bbmd_as_foreign_device() {
    let mut bbmd = simulated_bbmd(300, 47980, true).await;

    let connection = connect_via(47981, Some(at(47980))).await.unwrap();
    let report = scan::run(
        &connection.client,
        Duration::from_millis(500),
        Vec::new(),
        &|_| {},
    )
    .await
    .unwrap();

    connection.close().await.unwrap();
    bbmd.stop().await.unwrap();
    assert_eq!(report.devices.len(), 1, "{:?}", report.devices);
    assert_eq!(report.devices[0].instance, 300);
    assert!(report.devices[0].answered_who_is);
}

#[tokio::test]
async fn says_so_when_the_bbmd_refuses_foreign_devices() {
    let mut bbmd = simulated_bbmd(301, 47982, false).await;

    let result = connect_via(47983, Some(at(47982))).await;

    bbmd.stop().await.unwrap();
    let error = result.err().expect("registration must fail").to_string();
    assert!(error.contains("refused"), "{error}");
}

/// The tables of a BBMD, read only: its own BDT row, and this client in its FDT because it
/// registered as a foreign device.
#[tokio::test]
async fn reads_the_tables_of_a_bbmd() {
    let mut bbmd_device = simulated_bbmd(302, 47986, true).await;

    let connection = connect_via(47987, Some(at(47986))).await.unwrap();
    let tables = bbmd::read_tables(&connection.client, at(47986)).await;

    connection.close().await.unwrap();
    bbmd_device.stop().await.unwrap();
    assert!(tables.is_bbmd());
    let peers = tables.broadcast.expect("the BDT");
    assert!(
        peers.iter().any(|peer| peer.address == at(47986)),
        "{peers:?}"
    );
    let foreign = tables.foreign.expect("the FDT");
    let registered = foreign
        .iter()
        .find(|entry| entry.address.port() == 47987)
        .unwrap_or_else(|| panic!("this client is missing: {foreign:?}"));
    assert_eq!(registered.ttl_s, 300);
    assert!(registered.seconds_left > 0, "{registered:?}");
}

/// A device that is no BBMD refuses both reads; that is an answer, not a failure.
#[tokio::test]
async fn says_so_when_the_device_is_no_bbmd() {
    let mut device = simulated_device(303, "AHU-01", 47988, Segmentation::BOTH, Vec::new()).await;

    let connection = connect(47989).await;
    let tables = bbmd::read_tables(&connection.client, at(47988)).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert!(!tables.is_bbmd());
    assert_eq!(tables.broadcast.err(), Some(bbmd::TableNote::NoBbmd));
    assert_eq!(tables.foreign.err(), Some(bbmd::TableNote::NoBbmd));
}

#[tokio::test]
async fn says_so_when_nothing_answers_at_the_bbmd_address() {
    let result = connect_via(47985, Some(at(47984))).await;

    let error = result.err().expect("registration must fail").to_string();
    assert!(error.contains("47984"), "{error}");
}
