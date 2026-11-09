//! A simulated BACnet device to try bacnetio without a plant:
//! `cargo run --example simulated_plant -- <IP of an adapter> [port]`, then search on that adapter.
//!
//! It carries a plant view of its own in three Structured View objects, as controllers with a
//! system view do, so the tree can be tried without such a controller.

use std::f32::consts::TAU;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject, AnalogValueObject};
use bacnet_objects::binary::{BinaryInputObject, BinaryOutputObject};
use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
use bacnet_objects::event_log::EventLogObject;
use bacnet_objects::group::StructuredViewObject;
use bacnet_objects::multistate::MultiStateValueObject;
use bacnet_objects::schedule::ScheduleObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_objects::trend::{TrendLogMultipleObject, TrendLogObject};
use bacnet_server::server::{BACnetServer, DccPolicy};
use bacnet_types::constructed::{
    BACnetDeviceObjectPropertyReference, BACnetEventLogRecord, BACnetLogMultipleRecord,
    BACnetLogRecord, BACnetTimeValue, EventLogDatum, EventNotificationRequest, LogData, LogDatum,
    LogValue,
};
use bacnet_types::enums::{
    EngineeringUnits, EventState, EventType, NotifyType, ObjectType, PropertyIdentifier,
    Segmentation,
};
use bacnet_types::primitives::{BACnetTimeStamp, Date, ObjectIdentifier, PropertyValue, Time};

const PORT: u16 = 47808;
/// How often the temperatures move, so live values, COV and the recording have something to show.
const MOVE_EVERY: Duration = Duration::from_secs(5);

fn objects() -> Vec<Box<dyn BACnetObject>> {
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let mut supply = AnalogInputObject::new(1, "Supply air temp", celsius).unwrap();
    supply.set_present_value(21.4);
    supply.set_description("Supply duct after heating coil");
    let mut outdoor = AnalogInputObject::new(2, "Outdoor air temp", celsius).unwrap();
    outdoor.set_present_value(8.2);
    let filter = BinaryInputObject::new(1, "Filter alarm").unwrap();
    let mut valve = AnalogOutputObject::new(1, "Heating valve", EngineeringUnits::PERCENT).unwrap();
    valve.set_relinquish_default(35.0).unwrap();
    let mut fan = BinaryOutputObject::new(1, "Supply fan enable").unwrap();
    for (property, text) in [
        (PropertyIdentifier::INACTIVE_TEXT, "Off"),
        (PropertyIdentifier::ACTIVE_TEXT, "On"),
    ] {
        fan.write_property(
            property,
            None,
            PropertyValue::CharacterString(text.into()),
            None,
        )
        .unwrap();
    }
    let mut setpoint = AnalogValueObject::new(1, "Supply air setpoint", celsius).unwrap();
    setpoint.set_relinquish_default(20.0).unwrap();
    let mut mode = MultiStateValueObject::new(1, "Operating mode", 4).unwrap();
    for (index, text) in ["Off", "Day", "Night", "Frost protection"]
        .into_iter()
        .enumerate()
    {
        mode.write_property(
            PropertyIdentifier::STATE_TEXT,
            Some(index as u32 + 1),
            PropertyValue::CharacterString(text.into()),
            None,
        )
        .unwrap();
    }
    let rooms = (1..=10).map(|room| {
        let mut sensor =
            AnalogInputObject::new(10 + room, format!("Room temp 2.{room:02}"), celsius).unwrap();
        sensor.set_present_value(20.5 + room as f32 / 10.0);
        Box::new(sensor) as Box<dyn BACnetObject>
    });
    let mut schedule = ScheduleObject::new(1, "Office hours", PropertyValue::Real(17.0)).unwrap();
    for day in 0..5 {
        let at = |hour| Time {
            hour,
            minute: 0,
            second: 0,
            hundredths: 0,
        };
        schedule
            .set_weekly_schedule(
                day,
                vec![
                    BACnetTimeValue {
                        time: at(6),
                        value: PropertyValue::Real(21.5),
                    },
                    BACnetTimeValue {
                        time: at(18),
                        value: PropertyValue::Real(17.0),
                    },
                ],
            )
            .unwrap();
    }
    let mut objects: Vec<Box<dyn BACnetObject>> = vec![
        Box::new(schedule),
        Box::new(supply),
        Box::new(outdoor),
        Box::new(filter),
        Box::new(valve),
        Box::new(fan),
        Box::new(setpoint),
        Box::new(mode),
    ];
    objects.extend(rooms);
    objects.push(Box::new(supply_air_log()));
    objects.push(Box::new(air_handling_log()));
    objects.push(Box::new(alarm_history()));
    objects.extend(plant_view());
    objects
}

fn at(object_type: ObjectType, instance: u32) -> ObjectIdentifier {
    ObjectIdentifier::new(object_type, instance).unwrap()
}

/// The device's own plant view: a root that holds the air handling and the rooms, as a
/// controller's system view does. The schedule and the trend log hang below none of them.
fn plant_view() -> Vec<Box<dyn BACnetObject>> {
    let mut root = StructuredViewObject::new(1, "AHU-01 plant").unwrap();
    root.add_subordinate(at(ObjectType::STRUCTURED_VIEW, 2), "Air handling")
        .unwrap();
    root.add_subordinate(at(ObjectType::STRUCTURED_VIEW, 3), "Rooms")
        .unwrap();
    let mut air = StructuredViewObject::new(2, "Air handling").unwrap();
    for (object_type, instance, annotation) in [
        (ObjectType::ANALOG_INPUT, 1, "Supply air"),
        (ObjectType::ANALOG_INPUT, 2, "Outdoor air"),
        (ObjectType::BINARY_INPUT, 1, "Filter"),
        (ObjectType::ANALOG_OUTPUT, 1, "Heating valve"),
        (ObjectType::BINARY_OUTPUT, 1, "Supply fan"),
        (ObjectType::ANALOG_VALUE, 1, "Setpoint"),
        (ObjectType::MULTI_STATE_VALUE, 1, "Operating mode"),
    ] {
        air.add_subordinate(at(object_type, instance), annotation)
            .unwrap();
    }
    let mut rooms = StructuredViewObject::new(3, "Rooms").unwrap();
    for room in 1..=10 {
        rooms
            .add_subordinate(
                at(ObjectType::ANALOG_INPUT, 10 + room),
                format!("Room 2.{room:02}"),
            )
            .unwrap();
    }
    vec![Box::new(root), Box::new(air), Box::new(rooms)]
}

/// The quarters of hour of the logged day: 1 November 2026, every 15 minutes.
const QUARTERS: u32 = 96;

fn logged_day() -> Date {
    Date {
        year: 126,
        month: 11,
        day: 1,
        day_of_week: 7,
    }
}

fn quarter_time(quarter: u32) -> Time {
    Time {
        hour: (quarter / 4) as u8,
        minute: (quarter % 4 * 15) as u8,
        second: 0,
        hundredths: 0,
    }
}

/// The supply air temperature over the logged day: lowest in the morning, highest in the evening.
fn supply_air_at(quarter: u32) -> f32 {
    let hours = quarter as f32 / 4.0;
    let value = 20.0 + 2.0 * ((hours - 6.0) / 24.0 * TAU).sin();
    (value * 10.0).round() / 10.0
}

/// A day of supply air temperatures every 15 minutes, as a controller's own trend log holds them.
fn supply_air_log() -> TrendLogObject {
    let mut log = TrendLogObject::new(1, "Supply air temp log", 1000).unwrap();
    for quarter in 0..QUARTERS {
        log.add_record(BACnetLogRecord {
            date: logged_day(),
            time: quarter_time(quarter),
            log_datum: LogDatum::RealValue(supply_air_at(quarter)),
            status_flags: None,
        })
        .unwrap();
    }
    log
}

/// The same day of the air handling in one log of several points, as many controllers keep it:
/// supply air, heating valve and supply fan.
fn air_handling_log() -> TrendLogMultipleObject {
    let mut log = TrendLogMultipleObject::new(1, "Air handling log", 1000).unwrap();
    for (object_type, instance) in [
        (ObjectType::ANALOG_INPUT, 1),
        (ObjectType::ANALOG_OUTPUT, 1),
        (ObjectType::BINARY_OUTPUT, 1),
    ] {
        log.add_property_reference(BACnetDeviceObjectPropertyReference::new_local(
            at(object_type, instance),
            PropertyIdentifier::PRESENT_VALUE.to_raw(),
        ))
        .unwrap();
    }
    for quarter in 0..QUARTERS {
        let supply = supply_air_at(quarter);
        let running = (6 * 4..22 * 4).contains(&quarter);
        let valve = if running {
            ((22.0 - supply) * 25.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        log.add_record(BACnetLogMultipleRecord {
            date: logged_day(),
            time: quarter_time(quarter),
            log_data: LogData::Values(vec![
                LogValue::RealValue(supply),
                LogValue::RealValue(valve.round()),
                LogValue::EnumValue(u64::from(running)),
            ]),
        })
        .unwrap();
    }
    log
}

/// What the device logged of its alarms in the logged night: the filter, then the supply air too
/// warm, acknowledged in the morning, and back to normal.
fn alarm_history() -> EventLogObject {
    let mut log = EventLogObject::new(1, "Alarm history", 100).unwrap();
    let night = [
        (
            2,
            10,
            (ObjectType::BINARY_INPUT, 1),
            EventState::OFFNORMAL,
            NotifyType::ALARM,
            "Filter dirty",
        ),
        (
            3,
            40,
            (ObjectType::ANALOG_INPUT, 1),
            EventState::HIGH_LIMIT,
            NotifyType::ALARM,
            "Supply air too warm",
        ),
        (
            4,
            5,
            (ObjectType::ANALOG_INPUT, 1),
            EventState::NORMAL,
            NotifyType::ALARM,
            "Supply air normal",
        ),
        (
            7,
            30,
            (ObjectType::ANALOG_INPUT, 1),
            EventState::NORMAL,
            NotifyType::ACK_NOTIFICATION,
            "",
        ),
    ];
    for (hour, minute, (object_type, instance), to, notify_type, text) in night {
        let time = Time {
            hour,
            minute,
            second: 0,
            hundredths: 0,
        };
        log.add_record(BACnetEventLogRecord {
            date: logged_day(),
            time,
            log_datum: EventLogDatum::Notification(EventNotificationRequest {
                process_identifier: 1,
                initiating_device_identifier: at(ObjectType::DEVICE, 1001),
                event_object_identifier: at(object_type, instance),
                timestamp: BACnetTimeStamp::Time(time),
                notification_class: 1,
                priority: 100,
                event_type: EventType::OUT_OF_RANGE,
                message_text: (!text.is_empty()).then(|| text.to_string()),
                notify_type,
                ack_required: true,
                from_state: if to == EventState::NORMAL {
                    EventState::HIGH_LIMIT
                } else {
                    EventState::NORMAL
                },
                to_state: to,
                event_values: None,
            }),
        })
        .unwrap();
    }
    log
}

/// The temperatures at `minutes` after the start, by Analog Input instance: the supply air swings
/// within three minutes, the outdoor air within ten, each room a little and out of step.
fn moving_temperatures(minutes: f32) -> Vec<(u32, f32)> {
    let swing = |middle: f32, height: f32, period: f32, shift: f32| {
        let value = middle + height * (TAU * (minutes / period + shift)).sin();
        (value * 10.0).round() / 10.0
    };
    let mut values = vec![
        (1, swing(21.4, 1.5, 3.0, 0.0)),
        (2, swing(8.2, 0.5, 10.0, 0.0)),
    ];
    values.extend((1..=10).map(|room| {
        let middle = 20.5 + room as f32 / 10.0;
        (10 + room, swing(middle, 0.3, 5.0, room as f32 / 10.0))
    }));
    values
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
    let identity = DeviceIdentity::new(1001, 42)
        .unwrap()
        .with_name("AHU-01")
        .with_segmentation(Segmentation::BOTH);
    let database = build_database_with_extra(&identity, objects()).unwrap();
    let server = BACnetServer::bip_builder()
        .interface(interface)
        .port(port)
        // bound to its own address, several simulators share one host and port
        .share_port_by_address(true)
        .broadcast_address(broadcast(interface))
        .device(identity)
        .database(database)
        // takes a stop of its sending without password, as many devices in plants do; the stop of
        // all communication it refuses, as devices after ASHRAE 135-2020 do
        .dcc_policy(DccPolicy::LegacyPermissive)
        .build()
        .await
        .expect("cannot start the simulated device");
    println!("Simulated device 1001 \"AHU-01\" on {interface}:{port}. Ctrl+C to stop.");
    let start = Instant::now();
    let mut tick = tokio::time::interval(MOVE_EVERY);
    loop {
        tick.tick().await;
        for (instance, value) in moving_temperatures(start.elapsed().as_secs_f32() / 60.0) {
            let sensor = at(ObjectType::ANALOG_INPUT, instance);
            let Err(error) = server
                .set_present_value_local(&sensor, PropertyValue::Real(value))
                .await
            else {
                continue;
            };
            // a sensor taken out of service keeps the value written to it, as a real one does
            let out_of_service = server
                .read_local(&sensor, PropertyIdentifier::OUT_OF_SERVICE, None)
                .await;
            if !matches!(out_of_service, Ok(PropertyValue::Boolean(true))) {
                eprintln!("cannot move {sensor:?}: {error}");
            }
        }
    }
}
