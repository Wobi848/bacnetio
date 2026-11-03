//! A simulated controller named like many real ones: objects as paths by plant part
//! ("/21/000/00/00/HZ_Kessel.01/temp"), about a hundred points, no segmentation, one sensor out of
//! service. To try the grouping and reading in parts:
//! `cargo run --example simulated_controller -- <IP of an adapter>`.

use std::net::Ipv4Addr;

use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject, AnalogValueObject};
use bacnet_objects::binary::{BinaryInputObject, BinaryOutputObject};
use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
use bacnet_objects::multistate::{MultiStateInputObject, MultiStateOutputObject};
use bacnet_objects::traits::BACnetObject;
use bacnet_server::server::BACnetServer;
use bacnet_types::enums::{EngineeringUnits, PropertyIdentifier, Segmentation};
use bacnet_types::primitives::PropertyValue;

const PORT: u16 = 47808;
const DEVICE: u32 = 21;
const PREFIX: &str = "/21/000/00/00";
const ZONES: u32 = 56;

fn path(part: &str, name: &str) -> String {
    format!("{PREFIX}/{part}/{name}")
}

fn state_texts(object: &mut dyn BACnetObject, texts: &[&str]) {
    for (index, text) in texts.iter().enumerate() {
        object
            .write_property(
                PropertyIdentifier::STATE_TEXT,
                Some(index as u32 + 1),
                PropertyValue::CharacterString((*text).into()),
                None,
            )
            .unwrap();
    }
}

fn system_part() -> Vec<Box<dyn BACnetObject>> {
    let mut state = MultiStateInputObject::new(1, path("SY_Datsi.01", "state"), 3).unwrap();
    state_texts(&mut state, &["Off", "Running", "Fault"]);
    state.set_present_value(2);
    let mut progress =
        AnalogInputObject::new(1, path("SY_Datsi.01", "procBar"), EngineeringUnits::PERCENT)
            .unwrap();
    progress.set_present_value(100.0);
    let mut command = MultiStateOutputObject::new(1, path("SY_Datsi.01", "command"), 3).unwrap();
    state_texts(&mut command, &["None", "Save", "Restore"]);
    // long names as real controllers have them, for the layout tests
    let mut supply = AnalogValueObject::new(
        9,
        path(
            "LU_Lueftungsanlage_Bueroflaeche_Nord_Obergeschoss.01",
            "Zuluft_Temperatur_nach_Waermerueckgewinnung",
        ),
        EngineeringUnits::DEGREES_CELSIUS,
    )
    .unwrap();
    supply.set_description(
        "Zulufttemperatur nach der Wärmerückgewinnung, gemessen im Kanal vor dem Nachheizregister",
    );
    vec![
        Box::new(state),
        Box::new(progress),
        Box::new(command),
        Box::new(supply),
    ]
}

fn boiler(number: u32) -> Vec<Box<dyn BACnetObject>> {
    let part = format!("HZ_Kessel.{number:02}");
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let mut temp = AnalogInputObject::new(100 + number, path(&part, "temp"), celsius).unwrap();
    temp.set_present_value(64.5 + number as f32);
    let setpoint = AnalogValueObject::new(100 + number, path(&part, "setpoint"), celsius).unwrap();
    let pump = BinaryOutputObject::new(100 + number, path(&part, "pump")).unwrap();
    let alarm = BinaryInputObject::new(100 + number, path(&part, "alarm")).unwrap();
    vec![
        Box::new(temp),
        Box::new(setpoint),
        Box::new(pump),
        Box::new(alarm),
    ]
}

fn zone(number: u32) -> Vec<Box<dyn BACnetObject>> {
    let part = format!("RA_Zone.{number:02}");
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let mut temp = AnalogInputObject::new(200 + number, path(&part, "temp"), celsius).unwrap();
    temp.set_present_value(20.0 + number as f32 / 10.0);
    if number == 7 {
        temp.write_property(
            PropertyIdentifier::OUT_OF_SERVICE,
            None,
            PropertyValue::Boolean(true),
            None,
        )
        .unwrap();
    }
    let setpoint = AnalogValueObject::new(200 + number, path(&part, "setpoint"), celsius).unwrap();
    let valve = AnalogOutputObject::new(
        200 + number,
        path(&part, "valve"),
        EngineeringUnits::PERCENT,
    )
    .unwrap();
    vec![Box::new(temp), Box::new(setpoint), Box::new(valve)]
}

fn objects() -> Vec<Box<dyn BACnetObject>> {
    let mut objects = system_part();
    objects.extend((1..=2).flat_map(boiler));
    objects.extend((1..=ZONES).flat_map(zone));
    objects
}

/// The broadcast address a simulator bound to its own address may use: on loopback the address
/// itself, elsewhere the limited broadcast.
fn broadcast(interface: Ipv4Addr) -> Ipv4Addr {
    if interface.is_loopback() {
        interface
    } else {
        Ipv4Addr::BROADCAST
    }
}

#[tokio::main]
async fn main() {
    let interface: Ipv4Addr = std::env::args()
        .nth(1)
        .and_then(|text| text.parse().ok())
        .expect("give the IP of an adapter, e.g. 192.168.1.10");
    let port: u16 = std::env::args()
        .nth(2)
        .map(|text| text.parse().expect("the port is a number"))
        .unwrap_or(PORT);
    let identity = DeviceIdentity::new(DEVICE, 42)
        .unwrap()
        .with_name("DDC4000_#21")
        .with_segmentation(Segmentation::NONE);
    let database = build_database_with_extra(&identity, objects()).unwrap();
    let _server = BACnetServer::bip_builder()
        .interface(interface)
        .port(port)
        // bound to its own address, several simulators share one host and port
        .share_port_by_address(true)
        .broadcast_address(broadcast(interface))
        .device(identity)
        .database(database)
        .build()
        .await
        .expect("cannot start the simulated controller");
    println!("Simulated controller {DEVICE} on {interface}:{port}. Ctrl+C to stop.");
    std::future::pending::<()>().await;
}
