//! Data points in plain words: "Analog input 3", "21.4 °C", "fault, overridden".

use bacnet_types::constructed::BACnetAddress;
use bacnet_types::enums::{EngineeringUnits, ObjectType};
use bacnet_types::primitives::PropertyValue;

use crate::address::Address;
use crate::points::{Point, StatusFlags};

mod dates;
mod entries;
mod objects;

pub use dates::{date_text, date_time_text, time_text};
pub use entries::entry_action;
pub use objects::{object_text, object_type_text, parse_object};

/// Symbols for units common in building automation; others are spelled out.
const UNIT_SYMBOLS: &[(&str, &str)] = &[
    ("NO_UNITS", ""),
    ("DEGREES_CELSIUS", "°C"),
    ("DEGREES_FAHRENHEIT", "°F"),
    ("DEGREES_KELVIN", "K"),
    ("DELTA_DEGREES_KELVIN", "K"),
    ("PERCENT", "%"),
    ("PERCENT_RELATIVE_HUMIDITY", "%rH"),
    ("PASCALS", "Pa"),
    ("HECTOPASCALS", "hPa"),
    ("KILOPASCALS", "kPa"),
    ("BARS", "bar"),
    ("MILLIBARS", "mbar"),
    ("CUBIC_METERS", "m³"),
    ("CUBIC_METERS_PER_HOUR", "m³/h"),
    ("CUBIC_METERS_PER_SECOND", "m³/s"),
    ("LITERS", "l"),
    ("LITERS_PER_SECOND", "l/s"),
    ("LITERS_PER_MINUTE", "l/min"),
    ("LITERS_PER_HOUR", "l/h"),
    ("METERS_PER_SECOND", "m/s"),
    ("WATTS", "W"),
    ("KILOWATTS", "kW"),
    ("MEGAWATTS", "MW"),
    ("WATT_HOURS", "Wh"),
    ("KILOWATT_HOURS", "kWh"),
    ("MEGAWATT_HOURS", "MWh"),
    ("VOLTS", "V"),
    ("MILLIVOLTS", "mV"),
    ("AMPERES", "A"),
    ("MILLIAMPERES", "mA"),
    ("HERTZ", "Hz"),
    ("REVOLUTIONS_PER_MINUTE", "rpm"),
    ("PARTS_PER_MILLION", "ppm"),
    ("LUXES", "lx"),
    ("SECONDS", "s"),
    ("MINUTES", "min"),
    ("HOURS", "h"),
    ("DAYS", "d"),
];

pub fn unit_text(units: EngineeringUnits) -> String {
    let Some(name) = units.name() else {
        return format!("unit {}", units.to_raw());
    };
    match UNIT_SYMBOLS.iter().find(|(known, _)| *known == name) {
        Some((_, symbol)) => (*symbol).to_string(),
        None => name.to_lowercase().replace('_', " "),
    }
}

/// The present value with its unit, e.g. "21.4 °C", "active", "state 2".
pub fn value_text(point: &Point) -> String {
    match &point.value {
        Some(value) => value_with_unit(point.object.object_type(), value, point.units),
        None => String::new(),
    }
}

/// The present value without unit.
pub fn plain_value(point: &Point) -> String {
    match &point.value {
        Some(value) => plain_text(point.object.object_type(), value),
        None => String::new(),
    }
}

/// The present value as a person reads it: a multi-state object's state by its text.
pub fn shown_value(point: &Point) -> String {
    state_word(point).map_or_else(|| value_text(point), str::to_owned)
}

/// [`shown_value`] without unit.
pub fn shown_plain_value(point: &Point) -> String {
    state_word(point).map_or_else(|| plain_value(point), str::to_owned)
}

fn state_word(point: &Point) -> Option<&str> {
    let Some(PropertyValue::Unsigned(state)) = point.value else {
        return None;
    };
    let index = usize::try_from(state.checked_sub(1)?).ok()?;
    point
        .state_texts
        .get(index)
        .map(String::as_str)
        .filter(|text| !text.is_empty())
}

/// Any value of an object in words with its unit, such as a slot of the priority array.
pub fn value_with_unit(
    object_type: ObjectType,
    value: &PropertyValue,
    units: Option<EngineeringUnits>,
) -> String {
    let text = plain_text(object_type, value);
    match units.map(unit_text) {
        Some(unit) if is_number(object_type, value) && !unit.is_empty() => {
            format!("{text} {unit}")
        }
        _ => text,
    }
}

/// Any value of an object in words, without unit.
pub fn plain_text(object_type: ObjectType, value: &PropertyValue) -> String {
    match value {
        PropertyValue::Real(value) => number_text(f64::from(*value)),
        PropertyValue::Double(value) => number_text(*value),
        PropertyValue::Unsigned(value) if is_multistate(object_type) => format!("state {value}"),
        PropertyValue::Unsigned(value) => value.to_string(),
        PropertyValue::Signed(value) => value.to_string(),
        PropertyValue::Enumerated(value) if is_binary(object_type) => {
            if *value == 0 { "inactive" } else { "active" }.to_string()
        }
        PropertyValue::Enumerated(value) => value.to_string(),
        PropertyValue::Boolean(value) => value.to_string(),
        PropertyValue::CharacterString(text) => text.clone(),
        PropertyValue::Null => "null".to_string(),
        PropertyValue::ObjectIdentifier(object) => object_text(*object),
        PropertyValue::BitString { unused_bits, data } => bits_text(*unused_bits, data),
        PropertyValue::Date(date) => date_text(date),
        PropertyValue::Time(time) => time_text(time),
        PropertyValue::OctetString(bytes) => hex(bytes),
        PropertyValue::List(values) => values
            .iter()
            .map(|value| plain_text(object_type, value))
            .collect::<Vec<_>>()
            .join(", "),
        _ => "(structured value)".to_string(),
    }
}

/// What a device answered, in words, for messages about an answer that does not belong where it
/// arrived: "the number 3" instead of Rust's own `Real(3.0)`. It names the kind as BACnet does,
/// because that is what a technician compares against the device's documentation.
pub fn value_kind_text(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Real(number) => format!("the number {}", number_text(f64::from(*number))),
        PropertyValue::Double(number) => format!("the number {}", number_text(*number)),
        PropertyValue::Unsigned(number) => format!("the whole number {number}"),
        PropertyValue::Signed(number) => format!("the whole number {number}"),
        PropertyValue::Enumerated(number) => format!("the choice {number}"),
        PropertyValue::Boolean(flag) => format!("the yes/no value {flag}"),
        PropertyValue::CharacterString(text) => format!("the text \"{text}\""),
        PropertyValue::Null => "no value".to_string(),
        PropertyValue::BitString { unused_bits, data } => {
            format!("the bits {}", bits_text(*unused_bits, data))
        }
        PropertyValue::Date(date) => format!("the date {}", date_text(date)),
        PropertyValue::Time(time) => format!("the time {}", time_text(time)),
        PropertyValue::OctetString(bytes) => format!("the bytes {}", hex(bytes)),
        PropertyValue::ObjectIdentifier(object) => format!("the object {}", object_text(*object)),
        PropertyValue::List(values) => format!("a list of {} values", values.len()),
        _ => "a structured value".to_string(),
    }
}

/// The bits of a bit string, first bit first, without the unused ones at the end.
pub fn bits(unused_bits: u8, data: &[u8]) -> Vec<bool> {
    let count = (data.len() * 8).saturating_sub(usize::from(unused_bits));
    (0..count)
        .map(|bit| data[bit / 8] & (0x80 >> (bit % 8)) != 0)
        .collect()
}

/// "0110": one character per bit, first bit first.
fn bits_text(unused_bits: u8, data: &[u8]) -> String {
    bits(unused_bits, data)
        .into_iter()
        .map(|set| if set { '1' } else { '0' })
        .collect()
}

fn is_number(object_type: ObjectType, value: &PropertyValue) -> bool {
    match value {
        PropertyValue::Real(_) | PropertyValue::Double(_) | PropertyValue::Signed(_) => true,
        PropertyValue::Unsigned(_) => !is_multistate(object_type),
        _ => false,
    }
}

/// The status flags that are set, in words; empty when all is well.
pub fn status_text(status: Option<StatusFlags>) -> String {
    let Some(status) = status else {
        return String::new();
    };
    let words: Vec<&str> = [
        (status.in_alarm, "alarm"),
        (status.fault, "fault"),
        (status.overridden, "overridden"),
        (status.out_of_service, "out of service"),
    ]
    .into_iter()
    .filter_map(|(set, word)| set.then_some(word))
    .collect();
    words.join(", ")
}

/// Where a notification goes: the IP on the local network, else the network and the station's MAC,
/// as a device behind a router is addressed.
pub fn bacnet_address_text(address: &BACnetAddress) -> String {
    let network = match address.network_number {
        0 => String::new(),
        number => format!("network {number} "),
    };
    if address.mac_address.is_empty() {
        return format!("{network}broadcast");
    }
    match Address::from_mac(&address.mac_address) {
        Some(found) => format!("{network}{}", found.shown()),
        None => format!("{network}MAC {}", hex(&address.mac_address)),
    }
}

/// A device address: the IP alone on the standard port, else IP:PORT; an MS/TP station as
/// `MS/TP 5`.
pub fn address_text(address: impl Into<Address>) -> String {
    address.into().shown()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

/// At most two decimals, without trailing zeros: 21.40 → "21.4", 3.0 → "3".
fn number_text(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0" } else { text }.to_string()
}

pub(super) fn sentence_case(constant: &str) -> String {
    let lower = constant.to_lowercase().replace('_', " ");
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => lower,
    }
}

fn is_binary(object_type: ObjectType) -> bool {
    [
        ObjectType::BINARY_INPUT,
        ObjectType::BINARY_OUTPUT,
        ObjectType::BINARY_VALUE,
    ]
    .contains(&object_type)
}

pub(crate) fn is_multistate(object_type: ObjectType) -> bool {
    [
        ObjectType::MULTI_STATE_INPUT,
        ObjectType::MULTI_STATE_OUTPUT,
        ObjectType::MULTI_STATE_VALUE,
    ]
    .contains(&object_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::primitives::{Date, ObjectIdentifier, Time};

    fn point(object_type: ObjectType, value: PropertyValue) -> Point {
        let mut point = Point::new(ObjectIdentifier::new(object_type, 3).unwrap());
        point.value = Some(value);
        point
    }

    #[test]
    fn object_names_in_words() {
        let object = ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 3).unwrap();
        assert_eq!(object_text(object), "Analog input 3");
        assert_eq!(
            object_type_text(ObjectType::MULTI_STATE_VALUE),
            "Multi state value"
        );
    }

    #[test]
    fn units_as_symbols_or_words() {
        assert_eq!(unit_text(EngineeringUnits::DEGREES_CELSIUS), "°C");
        assert_eq!(unit_text(EngineeringUnits::NO_UNITS), "");
        assert_eq!(unit_text(EngineeringUnits::JOULES), "joules");
    }

    #[test]
    fn analog_value_with_unit() {
        let mut point = point(ObjectType::ANALOG_INPUT, PropertyValue::Real(21.40));
        point.units = Some(EngineeringUnits::DEGREES_CELSIUS);
        assert_eq!(value_text(&point), "21.4 °C");
        point.units = Some(EngineeringUnits::NO_UNITS);
        point.value = Some(PropertyValue::Real(3.0));
        assert_eq!(value_text(&point), "3");
    }

    #[test]
    fn binary_and_multistate_values_in_words() {
        let binary = point(ObjectType::BINARY_OUTPUT, PropertyValue::Enumerated(1));
        assert_eq!(value_text(&binary), "active");
        let binary = point(ObjectType::BINARY_INPUT, PropertyValue::Enumerated(0));
        assert_eq!(value_text(&binary), "inactive");
        let mut multistate = point(ObjectType::MULTI_STATE_VALUE, PropertyValue::Unsigned(2));
        assert_eq!(value_text(&multistate), "state 2");
        assert_eq!(shown_value(&multistate), "state 2");
        multistate.state_texts = vec!["Off".into(), "Running".into()];
        assert_eq!(shown_value(&multistate), "Running");
        assert_eq!(value_text(&multistate), "state 2");
        multistate.value = Some(PropertyValue::Unsigned(3));
        assert_eq!(shown_value(&multistate), "state 3");
    }

    /// Messages about an answer that does not fit name it in words, never as Rust prints it.
    #[test]
    fn what_a_device_answered_in_words() {
        assert_eq!(value_kind_text(&PropertyValue::Real(3.0)), "the number 3");
        assert_eq!(
            value_kind_text(&PropertyValue::Unsigned(3)),
            "the whole number 3"
        );
        assert_eq!(
            value_kind_text(&PropertyValue::CharacterString("AHU".into())),
            "the text \"AHU\""
        );
        assert_eq!(value_kind_text(&PropertyValue::Null), "no value");
        assert_eq!(
            value_kind_text(&PropertyValue::ObjectIdentifier(
                ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 3).unwrap()
            )),
            "the object Analog input 3"
        );
        assert_eq!(
            value_kind_text(&PropertyValue::Time(Time {
                hour: 6,
                minute: 30,
                second: 0,
                hundredths: 0,
            })),
            "the time 06:30:00"
        );
    }

    /// Rust's Debug notation prints `WEEKLY_SCHEDULE` or `Real(3.0)`; neither belongs on a page or
    /// in the CLI's output. The words are in this module and in property_text.rs.
    #[test]
    fn no_rust_notation_in_what_people_read() {
        fn sources(folder: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(folder)
                .expect("src is readable")
                .flatten()
            {
                let path = entry.path();
                if path.is_dir() {
                    sources(&path, found);
                } else if path.extension().is_some_and(|kind| kind == "rs") {
                    found.push(path);
                }
            }
        }
        let mut files = Vec::new();
        sources(std::path::Path::new("src"), &mut files);
        assert!(files.len() > 20, "found {} files", files.len());
        // the three characters apart, so that this test does not find itself
        let notation: String = [':', '?', '}'].iter().collect();
        let with_notation: Vec<_> = files
            .iter()
            .filter(|path| {
                std::fs::read_to_string(path)
                    .expect("source is readable")
                    .contains(&notation)
            })
            .map(|path| path.display().to_string())
            .collect();
        assert_eq!(with_notation, Vec::<String>::new());
    }

    #[test]
    fn numbers_without_noise() {
        assert_eq!(number_text(21.456), "21.46");
        assert_eq!(number_text(-0.001), "0");
        assert_eq!(number_text(100.0), "100");
    }

    #[test]
    fn status_only_names_what_is_set() {
        assert_eq!(status_text(None), "");
        assert_eq!(status_text(Some(StatusFlags::default())), "");
        let status = StatusFlags {
            fault: true,
            overridden: true,
            ..StatusFlags::default()
        };
        assert_eq!(status_text(Some(status)), "fault, overridden");
    }

    #[test]
    fn bits_dates_and_times_in_words() {
        assert_eq!(bits_text(4, &[0b0110_0000]), "0110");
        let date = Date {
            year: 126,
            month: 11,
            day: 1,
            day_of_week: 7,
        };
        assert_eq!(date_text(&date), "2026-11-01");
        let time = Time {
            hour: 9,
            minute: 30,
            second: 0xFF,
            hundredths: 0,
        };
        assert_eq!(time_text(&time), "09:30:*");
        assert_eq!(
            plain_text(
                ObjectType::ANALOG_INPUT,
                &PropertyValue::ApplicationData(vec![1])
            ),
            "(structured value)"
        );
    }

    #[test]
    fn objects_as_typed() {
        let valve = parse_object("AO:1").unwrap();
        assert_eq!(valve.object_type(), ObjectType::ANALOG_OUTPUT);
        assert_eq!(valve.instance_number(), 1);
        assert_eq!(
            parse_object("19:4").unwrap().object_type(),
            ObjectType::MULTI_STATE_VALUE
        );
        assert!(parse_object("valve").is_err());
        assert!(parse_object("xx:1").is_err());
    }
}
