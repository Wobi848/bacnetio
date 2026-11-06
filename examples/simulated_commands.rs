//! A simulated controller whose outputs are held at several priorities at once, to try the
//! priority grid and the KBOB check of table 10 without a plant:
//! `cargo run --example simulated_commands -- <IP of an adapter> [port]`.
//!
//! Some commands sit where KBOB gives the priority a use (8 manual from the MBE, 16 automatic,
//! 2 fire control, 14 time programs, 6 minimum on/off, 12 central functions), some at priorities
//! KBOB leaves free (9, 13). One schedule writes at 14 as KBOB asks, one at 16.

use std::net::Ipv4Addr;

use bacnet_objects::analog::{AnalogOutputObject, AnalogValueObject};
use bacnet_objects::binary::{BinaryOutputObject, BinaryValueObject};
use bacnet_objects::command_source::CommandOrigin;
use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
use bacnet_objects::schedule::ScheduleObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_server::server::BACnetServer;
use bacnet_types::enums::{EngineeringUnits, ObjectType, PropertyIdentifier, Segmentation};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

const PORT: u16 = 47808;
const INSTANCE: u32 = 1200;
const NAME: &str = "Commands-01";
const ACTIVE: PropertyValue = PropertyValue::Enumerated(1);
const INACTIVE: PropertyValue = PropertyValue::Enumerated(0);

/// Writes each value to the present value at its priority, as other systems would have.
fn hold(object: &mut dyn BACnetObject, commands: &[(u8, PropertyValue)]) {
    let origin = CommandOrigin::Local {
        owner_device: ObjectIdentifier::new(ObjectType::DEVICE, INSTANCE).unwrap(),
        initiating_object: None,
    };
    for (priority, value) in commands {
        object
            .write_property_from(
                PropertyIdentifier::PRESENT_VALUE,
                None,
                value.clone(),
                Some(*priority),
                &origin,
            )
            .unwrap();
    }
}

fn objects() -> Vec<Box<dyn BACnetObject>> {
    let percent = EngineeringUnits::PERCENT;
    let mut heating = AnalogOutputObject::new(1, "Heating valve", percent).unwrap();
    heating.set_relinquish_default(0.0).unwrap();
    hold(
        &mut heating,
        &[
            (8, PropertyValue::Real(40.0)),
            (16, PropertyValue::Real(20.0)),
        ],
    );
    let mut cooling = AnalogOutputObject::new(2, "Cooling valve", percent).unwrap();
    cooling.set_relinquish_default(0.0).unwrap();
    // priority 9 is free in KBOB table 10: allowed only by agreement with the owner
    hold(&mut cooling, &[(9, PropertyValue::Real(30.0))]);
    let mut fan = BinaryOutputObject::new(1, "Supply fan").unwrap();
    hold(&mut fan, &[(2, INACTIVE), (14, ACTIVE)]);
    let mut pump = BinaryOutputObject::new(2, "Heating pump").unwrap();
    hold(&mut pump, &[(6, ACTIVE), (16, INACTIVE)]);
    let mut night = BinaryValueObject::new(1, "Night mode").unwrap();
    // priority 13 is free in KBOB table 10 as well
    hold(&mut night, &[(13, ACTIVE)]);
    let mut setpoint =
        AnalogValueObject::new(1, "Room setpoint", EngineeringUnits::DEGREES_CELSIUS).unwrap();
    setpoint.set_relinquish_default(20.0).unwrap();
    hold(&mut setpoint, &[(12, PropertyValue::Real(21.0))]);
    let mut office = ScheduleObject::new(1, "Office hours", INACTIVE).unwrap();
    office.set_priority_for_writing(14).unwrap();
    // the library's default, 16: KBOB asks 14 for time programs
    let cleaning = ScheduleObject::new(2, "Cleaning light", INACTIVE).unwrap();
    vec![
        Box::new(heating),
        Box::new(cooling),
        Box::new(fan),
        Box::new(pump),
        Box::new(night),
        Box::new(setpoint),
        Box::new(office),
        Box::new(cleaning),
    ]
}

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
    let identity = DeviceIdentity::new(INSTANCE, 42)
        .unwrap()
        .with_name(NAME)
        .with_segmentation(Segmentation::BOTH);
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
        .expect("cannot start the simulated device");
    println!("Simulated device {INSTANCE} \"{NAME}\" on {interface}:{port}. Ctrl+C to stop.");
    std::future::pending::<()>().await;
}
