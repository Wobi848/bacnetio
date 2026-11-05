//! A simulated automation station that meets the KBOB profile AS-CH, to try the KBOB plugin:
//! `cargo run --example simulated_kbob -- <IP of an adapter> [port]`.
//!
//! The simulator library leaves out properties KBOB asks for (Location, the time synchronisation
//! recipients, Device_Type, Resolution …). A hull around each object adds them, so every rule of
//! the plugin is met. Not part of the browser tests' plant: the KBOB test starts it itself.

use std::borrow::Cow;
use std::net::Ipv4Addr;

use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject, AnalogValueObject};
use bacnet_objects::binary::{BinaryInputObject, BinaryOutputObject};
use bacnet_objects::database::ObjectDatabase;
use bacnet_objects::device::{DeviceConfig, DeviceObject};
use bacnet_objects::device_identity::DeviceIdentity;
use bacnet_objects::multistate::MultiStateValueObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_server::server::BACnetServer;
use bacnet_types::enums::{EngineeringUnits, PropertyIdentifier, Segmentation};
use bacnet_types::error::Error;
use bacnet_types::primitives::{Date, ObjectIdentifier, PropertyValue, Time};

const PORT: u16 = 47808;
const INSTANCE: u32 = 1100;

/// An object with the properties the library leaves out; they read as given, writes go inside.
struct Hull<O> {
    inner: O,
    more: Vec<(PropertyIdentifier, PropertyValue)>,
}

impl<O: BACnetObject> Hull<O> {
    fn new(inner: O, more: Vec<(PropertyIdentifier, PropertyValue)>) -> Box<Self> {
        Box::new(Hull { inner, more })
    }

    fn more(&self, property: PropertyIdentifier) -> Option<&PropertyValue> {
        self.more
            .iter()
            .find(|(known, _)| *known == property)
            .map(|(_, value)| value)
    }
}

impl<O: BACnetObject + 'static> BACnetObject for Hull<O> {
    fn device_authority_internal(&mut self) -> Option<bacnet_objects::device::DeviceAuthority<'_>> {
        self.inner.device_authority_internal()
    }

    fn object_identifier(&self) -> ObjectIdentifier {
        self.inner.object_identifier()
    }

    fn object_name(&self) -> &str {
        self.inner.object_name()
    }

    fn read_property(
        &self,
        property: PropertyIdentifier,
        array_index: Option<u32>,
    ) -> Result<PropertyValue, Error> {
        match self.more(property) {
            Some(value) => Ok(value.clone()),
            None => self.inner.read_property(property, array_index),
        }
    }

    fn write_property(
        &mut self,
        property: PropertyIdentifier,
        array_index: Option<u32>,
        value: PropertyValue,
        priority: Option<u8>,
    ) -> Result<(), Error> {
        self.inner
            .write_property(property, array_index, value, priority)
    }

    /// Asked anew each time: the device lists its clock only once the server has bound one.
    fn property_list(&self) -> Cow<'static, [PropertyIdentifier]> {
        let mut list = self.inner.property_list().into_owned();
        for (property, _) in &self.more {
            if !list.contains(property) {
                list.push(*property);
            }
        }
        Cow::Owned(list)
    }

    fn bind_clock_internal(
        &mut self,
        clock: Option<std::sync::Arc<dyn bacnet_objects::clock::ClockReader>>,
    ) {
        self.inner.bind_clock_internal(clock);
    }

    fn is_writable_property(&self, property: PropertyIdentifier) -> bool {
        self.more(property).is_none() && self.inner.is_writable_property(property)
    }

    fn is_array_property(&self, property: PropertyIdentifier) -> bool {
        self.inner.is_array_property(property)
    }

    fn is_list_property(&self, property: PropertyIdentifier) -> bool {
        self.inner.is_list_property(property)
    }

    fn supports_cov(&self) -> bool {
        self.inner.supports_cov()
    }
}

fn text(value: &str) -> PropertyValue {
    PropertyValue::CharacterString(value.into())
}

/// 1 November 2026, 06:00: a date and time as BACnet sends them, date then time.
fn date_time() -> PropertyValue {
    PropertyValue::List(vec![
        PropertyValue::Date(Date {
            year: 126,
            month: 11,
            day: 1,
            day_of_week: 7,
        }),
        PropertyValue::Time(Time {
            hour: 6,
            minute: 0,
            second: 0,
            hundredths: 0,
        }),
    ])
}

fn empty_list() -> PropertyValue {
    PropertyValue::List(Vec::new())
}

fn device_more() -> Vec<(PropertyIdentifier, PropertyValue)> {
    vec![
        (PropertyIdentifier::LOCATION, text("Simulated plant room")),
        (
            PropertyIdentifier::DESCRIPTION,
            text("Automation station that meets KBOB AS-CH"),
        ),
        (
            PropertyIdentifier::TIME_SYNCHRONIZATION_RECIPIENTS,
            empty_list(),
        ),
        (
            PropertyIdentifier::UTC_TIME_SYNCHRONIZATION_RECIPIENTS,
            empty_list(),
        ),
        (
            PropertyIdentifier::RESTART_NOTIFICATION_RECIPIENTS,
            empty_list(),
        ),
        (PropertyIdentifier::TIME_OF_DEVICE_RESTART, date_time()),
    ]
}

/// What an input or output names about itself on top of the library's properties.
fn field(device_type: &str) -> Vec<(PropertyIdentifier, PropertyValue)> {
    vec![
        (PropertyIdentifier::DEVICE_TYPE, text(device_type)),
        (PropertyIdentifier::DESCRIPTION, text(device_type)),
    ]
}

fn analog(
    device_type: Option<&str>,
    update_interval: bool,
) -> Vec<(PropertyIdentifier, PropertyValue)> {
    let mut more = device_type.map(field).unwrap_or_default();
    more.extend([
        (
            PropertyIdentifier::MIN_PRES_VALUE,
            PropertyValue::Real(-50.0),
        ),
        (
            PropertyIdentifier::MAX_PRES_VALUE,
            PropertyValue::Real(150.0),
        ),
        (PropertyIdentifier::RESOLUTION, PropertyValue::Real(0.1)),
    ]);
    if update_interval {
        more.push((
            PropertyIdentifier::UPDATE_INTERVAL,
            PropertyValue::Unsigned(100),
        ));
    }
    if device_type.is_none() {
        more.push((PropertyIdentifier::DESCRIPTION, text("Setpoint")));
    }
    more
}

fn binary(device_type: &str, output: bool) -> Vec<(PropertyIdentifier, PropertyValue)> {
    let mut more = field(device_type);
    more.extend([
        (PropertyIdentifier::ACTIVE_TEXT, text("On")),
        (PropertyIdentifier::INACTIVE_TEXT, text("Off")),
        (PropertyIdentifier::CHANGE_OF_STATE_TIME, date_time()),
        (
            PropertyIdentifier::CHANGE_OF_STATE_COUNT,
            PropertyValue::Unsigned(12),
        ),
        (PropertyIdentifier::TIME_OF_STATE_COUNT_RESET, date_time()),
        (
            PropertyIdentifier::ELAPSED_ACTIVE_TIME,
            PropertyValue::Unsigned(3600),
        ),
        (PropertyIdentifier::TIME_OF_ACTIVE_TIME_RESET, date_time()),
    ]);
    if output {
        more.extend([
            (
                PropertyIdentifier::MINIMUM_OFF_TIME,
                PropertyValue::Unsigned(60),
            ),
            (
                PropertyIdentifier::MINIMUM_ON_TIME,
                PropertyValue::Unsigned(60),
            ),
        ]);
    }
    more
}

fn objects() -> Vec<Box<dyn BACnetObject>> {
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let mut temp = AnalogInputObject::new(1, "AS01_Supply_Temp", celsius).unwrap();
    temp.set_present_value(21.0);
    let mut valve =
        AnalogOutputObject::new(1, "AS01_Heating_Valve", EngineeringUnits::PERCENT).unwrap();
    valve.set_relinquish_default(0.0).unwrap();
    let mut setpoint = AnalogValueObject::new(1, "AS01_Supply_Setpoint", celsius).unwrap();
    setpoint.set_relinquish_default(20.0).unwrap();
    let alarm = BinaryInputObject::new(1, "AS01_Filter_Alarm").unwrap();
    let fan = BinaryOutputObject::new(1, "AS01_Supply_Fan").unwrap();
    let mut mode = MultiStateValueObject::new(1, "AS01_Operating_Mode", 3).unwrap();
    for (index, state) in ["Off", "Day", "Night"].into_iter().enumerate() {
        mode.write_property(
            PropertyIdentifier::STATE_TEXT,
            Some(index as u32 + 1),
            text(state),
            None,
        )
        .unwrap();
    }
    let mode_more = vec![
        (PropertyIdentifier::DESCRIPTION, text("Operating mode")),
        (
            PropertyIdentifier::ALARM_VALUES,
            PropertyValue::List(vec![PropertyValue::Unsigned(1)]),
        ),
        (PropertyIdentifier::FAULT_VALUES, empty_list()),
    ];
    vec![
        Hull::new(temp, analog(Some("PT1000 duct sensor"), true)),
        Hull::new(valve, analog(Some("0-10 V valve actuator"), false)),
        Hull::new(setpoint, analog(None, false)),
        Hull::new(alarm, binary("Differential pressure switch", false)),
        Hull::new(fan, binary("Fan relay", true)),
        Hull::new(mode, mode_more),
    ]
}

/// The database as the library builds it, with the device in its hull and backup and restore on.
fn database(identity: &DeviceIdentity) -> ObjectDatabase {
    let mut device = DeviceObject::new(DeviceConfig {
        identity: identity.clone(),
        firmware_revision: env!("CARGO_PKG_VERSION").into(),
        application_software_version: env!("CARGO_PKG_VERSION").into(),
        backup_and_restore: true,
        ..DeviceConfig::default()
    })
    .unwrap();
    device.set_services_supported(identity.services());
    device.set_device_uuid(identity.device_uuid());
    let objects = objects();
    let mut list = vec![device.object_identifier()];
    list.extend(objects.iter().map(|object| object.object_identifier()));
    device.set_object_list(list);
    let mut database = ObjectDatabase::new();
    database.add(Hull::new(device, device_more())).unwrap();
    for object in objects {
        database.add(object).unwrap();
    }
    database
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
        .with_name("KBOB-AS-01")
        .with_segmentation(Segmentation::BOTH);
    let server = BACnetServer::bip_builder()
        .interface(interface)
        .port(port)
        // bound to its own address, several simulators share one host and port
        .share_port_by_address(true)
        .broadcast_address(broadcast(interface))
        .device(identity.clone())
        .database(database(&identity))
        .build()
        .await
        .expect("cannot start the simulated device");
    println!("Simulated device {INSTANCE} \"KBOB-AS-01\" on {interface}:{port}. Ctrl+C to stop.");
    let _keep = server;
    std::future::pending::<()>().await;
}
