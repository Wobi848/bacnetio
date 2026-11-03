//! Simulated devices and a client, talking over real UDP on the loopback interface.
//!
//! A Who-Is goes to the client's own port, so devices on other ports never hear it: like a
//! firewall that drops broadcast answers, only asking directly finds them.

// each test file uses only some of these helpers
#![allow(dead_code)]

use std::net::{Ipv4Addr, SocketAddrV4};

use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
use bacnet_objects::traits::BACnetObject;
use bacnet_server::server::BACnetServer;
use bacnet_transport::bbmd::ForeignDevicePolicy;
use bacnet_transport::bip::{BbmdOptions, BipMode, BipTransport};
use bacnet_types::enums::Segmentation;
use bacnetio::connection::{self, Connection, Endpoint};

pub const LOOPBACK: Ipv4Addr = Ipv4Addr::LOCALHOST;
const LOOPBACK_BROADCAST: Ipv4Addr = Ipv4Addr::new(127, 255, 255, 255);

pub async fn simulated_device(
    instance: u32,
    name: &str,
    port: u16,
    segmentation: Segmentation,
    objects: Vec<Box<dyn BACnetObject>>,
) -> BACnetServer<BipTransport> {
    let identity = DeviceIdentity::new(instance, 42)
        .unwrap()
        .with_name(name)
        .with_segmentation(segmentation);
    let database = build_database_with_extra(&identity, objects).unwrap();
    BACnetServer::bip_builder()
        .interface(LOOPBACK)
        .port(port)
        .broadcast_address(LOOPBACK_BROADCAST)
        .device(identity)
        .database(database)
        .build()
        .await
        .unwrap()
}

/// A device that is also a BBMD, accepting foreign devices or not.
pub async fn simulated_bbmd(
    instance: u32,
    port: u16,
    accepts_foreign_devices: bool,
) -> BACnetServer<BipTransport> {
    let identity = DeviceIdentity::new(instance, 42).unwrap().with_name("BBMD");
    let database = build_database_with_extra(&identity, Vec::new()).unwrap();
    let options = BbmdOptions {
        foreign_device_policy: accepts_foreign_devices.then(ForeignDevicePolicy::default),
        ..BbmdOptions::default()
    };
    BACnetServer::bip_builder()
        .interface(LOOPBACK)
        .port(port)
        .broadcast_address(LOOPBACK_BROADCAST)
        .mode(BipMode::Bbmd(options))
        .device(identity)
        .database(database)
        .build()
        .await
        .unwrap()
}

pub async fn connect(port: u16) -> Connection {
    connect_via(port, None).await.unwrap()
}

pub async fn connect_via(port: u16, bbmd: Option<SocketAddrV4>) -> anyhow::Result<Connection> {
    connection::open(&Endpoint {
        interface: LOOPBACK,
        broadcast: LOOPBACK_BROADCAST,
        port,
        bbmd,
    })
    .await
}

pub fn at(port: u16) -> SocketAddrV4 {
    SocketAddrV4::new(LOOPBACK, port)
}
