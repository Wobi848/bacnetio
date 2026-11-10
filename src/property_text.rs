//! Any property of an object in words, for the technical details: enumerations by name, units as
//! symbols, lists shortened.

use bacnet_encoding::constructed::decode_destination_list;
use bacnet_encoding::primitives::decode_timestamp_choice;
use bacnet_types::bitstring::{
    DaysOfWeek, EventTransitionBits, ObjectTypesSupported, ServicesSupported,
};
use bacnet_types::constructed::{BACnetDestination, BACnetRecipient};
use bacnet_types::enums::ResolvedEnum;
use bacnet_types::enums::{
    DeviceStatus, EngineeringUnits, EventState, ObjectType, Polarity, PropertyIdentifier,
    Reliability, Segmentation,
};
use bacnet_types::primitives::{BACnetTimeStamp, PropertyValue};

use crate::address::Link;
use crate::points::StatusFlags;
use crate::present::{
    bacnet_address_text, date_time_text, object_text, object_type_text, plain_text, status_text,
    time_text, unit_text, value_with_unit,
};

/// Lists longer than this are cut in the property view.
const SHOWN_LIST_ENTRIES: usize = 20;

/// "PRESENT_VALUE" → "present-value", as the standard writes property names.
pub fn property_name(property: PropertyIdentifier) -> String {
    match property.name() {
        Some(name) => name.to_lowercase().replace('_', "-"),
        None => format!("property {}", property.to_raw()),
    }
}

/// A property's values in words. `encoded` is the answer as the device sent it, needed for the
/// productions the flat list of values cannot express; `&[]` where there is none.
pub fn property_text(
    object_type: ObjectType,
    property: PropertyIdentifier,
    values: &[PropertyValue],
    units: Option<EngineeringUnits>,
    encoded: &[u8],
    link: Link,
) -> String {
    if property == PropertyIdentifier::RECIPIENT_LIST
        && let Some(text) = recipients_text(encoded, link)
    {
        return text;
    }
    shortened(
        values
            .iter()
            .take(SHOWN_LIST_ENTRIES)
            .map(|value| value_text(object_type, property, value, units))
            .collect(),
        values.len(),
    )
}

/// Each entry of a list or array property in words, one text each, cut after
/// [`SHOWN_LIST_ENTRIES`]: event time stamps, message texts, state texts. A list read as one
/// value is opened up.
pub fn property_items(
    object_type: ObjectType,
    property: PropertyIdentifier,
    values: &[PropertyValue],
    units: Option<EngineeringUnits>,
) -> Vec<String> {
    let entries = match values {
        [PropertyValue::List(inner)] => inner.as_slice(),
        _ => values,
    };
    entries
        .iter()
        .take(SHOWN_LIST_ENTRIES)
        .map(|value| value_text(object_type, property, value, units))
        .collect()
}

/// One value of a property in words.
fn value_text(
    object_type: ObjectType,
    property: PropertyIdentifier,
    value: &PropertyValue,
    units: Option<EngineeringUnits>,
) -> String {
    match (property, value) {
        (PropertyIdentifier::UNITS, PropertyValue::Enumerated(raw)) => {
            unit_text(EngineeringUnits::from_raw(*raw))
        }
        (PropertyIdentifier::RELIABILITY, PropertyValue::Enumerated(raw)) => {
            enumeration_text(Reliability::from_raw(*raw).name(), *raw)
        }
        (PropertyIdentifier::EVENT_STATE, PropertyValue::Enumerated(raw)) => {
            enumeration_text(EventState::from_raw(*raw).name(), *raw)
        }
        (PropertyIdentifier::POLARITY, PropertyValue::Enumerated(raw)) => {
            enumeration_text(Polarity::from_raw(*raw).name(), *raw)
        }
        (PropertyIdentifier::SYSTEM_STATUS, PropertyValue::Enumerated(raw)) => {
            enumeration_text(DeviceStatus::from_raw(*raw).name(), *raw)
        }
        (PropertyIdentifier::SEGMENTATION_SUPPORTED, PropertyValue::Enumerated(raw)) => {
            match u8::try_from(*raw) {
                Ok(small) => enumeration_text(Segmentation::from_raw(small).name(), *raw),
                Err(_) => raw.to_string(),
            }
        }
        (PropertyIdentifier::OBJECT_TYPE, PropertyValue::Enumerated(raw)) => {
            object_type_text(ObjectType::from_raw(*raw))
        }
        (PropertyIdentifier::STATUS_FLAGS, PropertyValue::BitString { data, .. }) => {
            match status_text(Some(StatusFlags::from_bits(data))) {
                text if text.is_empty() => "normal".to_string(),
                text => text,
            }
        }
        // what the device can do, the question behind most "it is slow" calls
        (
            PropertyIdentifier::PROTOCOL_SERVICES_SUPPORTED,
            PropertyValue::BitString { data, .. },
        ) => listed(
            ServicesSupported::from_bacnet(data)
                .iter()
                .map(|service| match service.name() {
                    Some(name) => name.to_lowercase().replace('_', "-"),
                    None => format!("service {}", service.to_raw()),
                })
                .collect(),
        ),
        (
            PropertyIdentifier::PROTOCOL_OBJECT_TYPES_SUPPORTED,
            PropertyValue::BitString { data, .. },
        ) => listed(
            ObjectTypesSupported::from_bacnet(data)
                .iter()
                .map(object_type_text)
                .collect(),
        ),
        (_, PropertyValue::ObjectIdentifier(object)) => object_text(*object),
        (
            PropertyIdentifier::PRESENT_VALUE
            | PropertyIdentifier::RELINQUISH_DEFAULT
            | PropertyIdentifier::COV_INCREMENT
            | PropertyIdentifier::MIN_PRES_VALUE
            | PropertyIdentifier::MAX_PRES_VALUE
            | PropertyIdentifier::HIGH_LIMIT
            | PropertyIdentifier::LOW_LIMIT
            | PropertyIdentifier::DEADBAND,
            value,
        ) => value_with_unit(object_type, value, units),
        (PropertyIdentifier::EVENT_TIME_STAMPS, PropertyValue::ApplicationData(bytes)) => {
            match decode_timestamp_choice(bytes, 0) {
                Ok((stamp, _)) => time_stamp_text(&stamp),
                Err(_) => plain_text(object_type, value),
            }
        }
        // the enumeration the property holds, where the property alone names it (notify type,
        // event type, …); the value of a binary or multi-state object stays with its object
        (_, PropertyValue::Enumerated(raw)) => {
            match ResolvedEnum::from_property(property, *raw).name() {
                Some(name) => enumeration_text(Some(name), *raw),
                None => plain_text(object_type, value),
            }
        }
        (_, value) => plain_text(object_type, value),
    }
}

/// When a transition last happened; "never" for the unset stamp, all wildcards or sequence 0.
fn time_stamp_text(stamp: &BACnetTimeStamp) -> String {
    const UNSET: u8 = 0xFF;
    match stamp {
        BACnetTimeStamp::DateTime { date, time } if date.year == UNSET && time.hour == UNSET => {
            "never".to_string()
        }
        BACnetTimeStamp::DateTime { date, time } => date_time_text(date, time),
        BACnetTimeStamp::Time(time) if time.hour == UNSET => "never".to_string(),
        BACnetTimeStamp::Time(time) => time_text(time),
        BACnetTimeStamp::SequenceNumber(0) => "never".to_string(),
        BACnetTimeStamp::SequenceNumber(number) => format!("#{number}"),
    }
}

/// `texts` joined, cut after [`SHOWN_LIST_ENTRIES`] of the `total` there are. A caller that knows
/// the whole list passes fewer texts than `total`, so that a long list is not put into words only
/// to be dropped.
fn shortened(mut texts: Vec<String>, total: usize) -> String {
    texts.truncate(SHOWN_LIST_ENTRIES);
    if total > SHOWN_LIST_ENTRIES {
        texts.push(format!("… {} more", total - SHOWN_LIST_ENTRIES));
    }
    texts.join(", ")
}

/// A list that is already in words, cut the same way.
fn listed(texts: Vec<String>) -> String {
    let total = texts.len();
    shortened(texts, total)
}

/// `Recipient_List` of a Notification Class: who gets its alarms, when and how. The entries are
/// `BACnetDestination` (Clause 21), which the flat list of values cannot express, so they are read
/// from the bytes the device sent; `None` where they do not decode.
fn recipients_text(encoded: &[u8], link: Link) -> Option<String> {
    let destinations = decode_destination_list(encoded).ok()?;
    if destinations.is_empty() {
        return Some("nobody".to_string());
    }
    Some(
        destinations
            .iter()
            .map(|destination| destination_text(destination, link))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn destination_text(destination: &BACnetDestination, link: Link) -> String {
    let who = match &destination.recipient {
        BACnetRecipient::Device(device) => object_text(*device),
        BACnetRecipient::Address(address) => bacnet_address_text(address, link),
    };
    let days = if destination.valid_days == DaysOfWeek::all() {
        "every day".to_string()
    } else if destination.valid_days.is_empty() {
        "no day".to_string()
    } else {
        destination
            .valid_days
            .iter_names()
            .map(|(name, _)| name[..3].to_lowercase())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let transitions: Vec<&str> = [
        (EventTransitionBits::TO_OFFNORMAL, "alarm"),
        (EventTransitionBits::TO_FAULT, "fault"),
        (EventTransitionBits::TO_NORMAL, "back to normal"),
    ]
    .into_iter()
    .filter(|(bit, _)| destination.transitions.contains(*bit))
    .map(|(_, word)| word)
    .collect();
    let what = if transitions.is_empty() {
        "nothing".to_string()
    } else {
        transitions.join(", ")
    };
    format!(
        "{who} · {days} {} to {} · {} · {what} · process {}",
        time_text(&destination.from_time),
        time_text(&destination.to_time),
        if destination.issue_confirmed_notifications {
            "confirmed"
        } else {
            "unconfirmed"
        },
        destination.process_identifier,
    )
}

fn enumeration_text(name: Option<&str>, raw: u32) -> String {
    match name {
        Some(name) => name.to_lowercase().replace('_', " "),
        None => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_encoding::constructed::encode_destination;
    use bacnet_types::enums::ServiceSupported;
    use bacnet_types::primitives::{Date, ObjectIdentifier, Time};
    use bytes::BytesMut;

    /// A bit string as a device sends it: the first bit of a byte is its highest.
    fn bits(set: &[u32]) -> PropertyValue {
        let mut data = vec![0u8; 16];
        for bit in set {
            data[(bit / 8) as usize] |= 0x80 >> (bit % 8);
        }
        PropertyValue::BitString {
            unused_bits: 0,
            data,
        }
    }

    /// The values as a device sends them, without the bytes only a recipient list needs.
    fn words(
        object_type: ObjectType,
        property: PropertyIdentifier,
        values: &[PropertyValue],
        units: Option<EngineeringUnits>,
    ) -> String {
        property_text(object_type, property, values, units, &[], Link::Ip)
    }

    fn stamp(stamp: &BACnetTimeStamp) -> PropertyValue {
        let mut bytes = BytesMut::new();
        bacnet_encoding::primitives::encode_timestamp_choice(&mut bytes, stamp).unwrap();
        PropertyValue::ApplicationData(bytes.to_vec())
    }

    #[test]
    fn event_time_stamps_and_lists_entry_by_entry() {
        let unset = BACnetTimeStamp::DateTime {
            date: Date {
                year: 0xFF,
                month: 0xFF,
                day: 0xFF,
                day_of_week: 0xFF,
            },
            time: Time {
                hour: 0xFF,
                minute: 0xFF,
                second: 0xFF,
                hundredths: 0xFF,
            },
        };
        let set = BACnetTimeStamp::DateTime {
            date: Date {
                year: 126,
                month: 11,
                day: 2,
                day_of_week: 1,
            },
            time: Time {
                hour: 14,
                minute: 3,
                second: 0,
                hundredths: 0,
            },
        };
        let values = [
            stamp(&set),
            stamp(&unset),
            stamp(&BACnetTimeStamp::SequenceNumber(7)),
        ];
        assert_eq!(
            property_items(
                ObjectType::ANALOG_INPUT,
                PropertyIdentifier::EVENT_TIME_STAMPS,
                &values,
                None
            ),
            ["2026-11-02 14:03:00", "never", "#7"]
        );
        let texts = [PropertyValue::List(vec![
            PropertyValue::CharacterString("too hot".into()),
            PropertyValue::CharacterString(String::new()),
        ])];
        assert_eq!(
            property_items(
                ObjectType::ANALOG_INPUT,
                PropertyIdentifier::EVENT_MESSAGE_TEXTS,
                &texts,
                None
            ),
            ["too hot", ""]
        );
    }

    /// Where the property alone names its enumeration, the number gets that name.
    #[test]
    fn enumerations_named_by_their_property() {
        assert_eq!(
            words(
                ObjectType::ANALOG_INPUT,
                PropertyIdentifier::NOTIFY_TYPE,
                &[PropertyValue::Enumerated(0)],
                None
            ),
            "alarm"
        );
        // a value outside the enumeration stays a number
        assert_eq!(
            words(
                ObjectType::ANALOG_INPUT,
                PropertyIdentifier::NOTIFY_TYPE,
                &[PropertyValue::Enumerated(77)],
                None
            ),
            "77"
        );
    }

    #[test]
    fn properties_in_words() {
        let ai = ObjectType::ANALOG_INPUT;
        assert_eq!(
            property_name(PropertyIdentifier::PRESENT_VALUE),
            "present-value"
        );
        assert_eq!(
            words(
                ai,
                PropertyIdentifier::RELIABILITY,
                &[PropertyValue::Enumerated(0)],
                None
            ),
            "no fault detected"
        );
        assert_eq!(
            words(
                ObjectType::DEVICE,
                PropertyIdentifier::SEGMENTATION_SUPPORTED,
                &[PropertyValue::Enumerated(0)],
                None
            ),
            "both"
        );
        assert_eq!(
            words(
                ai,
                PropertyIdentifier::COV_INCREMENT,
                &[PropertyValue::Real(0.5)],
                Some(EngineeringUnits::DEGREES_CELSIUS)
            ),
            "0.5 °C"
        );
        let long: Vec<PropertyValue> = (0..25).map(PropertyValue::Unsigned).collect();
        assert!(words(ai, PropertyIdentifier::PROPERTY_LIST, &long, None).ends_with("… 5 more"));
    }

    /// "Does this device do COV? does it do ReadRange?" is the question behind the technical
    /// details; a bit string of 48 characters does not answer it.
    #[test]
    fn what_a_device_can_do_in_words() {
        let services: Vec<u32> = [
            ServiceSupported::READ_PROPERTY,
            ServiceSupported::SUBSCRIBE_COV,
            ServiceSupported::READ_RANGE,
        ]
        .iter()
        .map(|service| u32::from(service.to_raw()))
        .collect();
        assert_eq!(
            words(
                ObjectType::DEVICE,
                PropertyIdentifier::PROTOCOL_SERVICES_SUPPORTED,
                &[bits(&services)],
                None,
            ),
            "subscribe-cov, read-property, read-range"
        );
        let types = [ObjectType::ANALOG_INPUT, ObjectType::ACCUMULATOR].map(ObjectType::to_raw);
        assert_eq!(
            words(
                ObjectType::DEVICE,
                PropertyIdentifier::PROTOCOL_OBJECT_TYPES_SUPPORTED,
                &[bits(&types)],
                None,
            ),
            "Analog input, Accumulator"
        );
    }

    /// "Who is this device supposed to send its alarms to" cannot be answered with
    /// "(structured value)".
    #[test]
    fn notification_recipients_in_words() {
        let destination = BACnetDestination {
            valid_days: DaysOfWeek::all(),
            from_time: Time {
                hour: 0,
                minute: 0,
                second: 0,
                hundredths: 0,
            },
            to_time: Time {
                hour: 23,
                minute: 59,
                second: 59,
                hundredths: 99,
            },
            recipient: BACnetRecipient::Device(
                ObjectIdentifier::new(ObjectType::DEVICE, 5).unwrap(),
            ),
            process_identifier: 1,
            issue_confirmed_notifications: true,
            transitions: EventTransitionBits::TO_OFFNORMAL | EventTransitionBits::TO_FAULT,
        };
        let mut encoded = BytesMut::new();
        encode_destination(&mut encoded, &destination).unwrap();
        let text = property_text(
            ObjectType::NOTIFICATION_CLASS,
            PropertyIdentifier::RECIPIENT_LIST,
            &[],
            None,
            &encoded,
            Link::Ip,
        );
        assert_eq!(
            text,
            "Device 5 · every day 00:00:00 to 23:59:59 · confirmed · alarm, fault · process 1"
        );
        // an empty list says so, and bytes that do not decode fall back to the values
        assert_eq!(
            property_text(
                ObjectType::NOTIFICATION_CLASS,
                PropertyIdentifier::RECIPIENT_LIST,
                &[],
                None,
                &[],
                Link::Ip,
            ),
            "nobody"
        );
    }
}
