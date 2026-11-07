//! Writing present values and releasing them again (concept section 6). Every write lands in the
//! journal; the values still held are derived from it, so nothing is left behind when leaving.

use crate::address::Address;
use std::collections::HashSet;
use std::time::SystemTime;

use bacnet_encoding::primitives::encode_property_value;
use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::error::Error;
use bacnet_types::primitives::{Date, ObjectIdentifier, PropertyValue, Time};
use bytes::BytesMut;

use crate::commands::{self, Restart};
use crate::connection::Client;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Write(PropertyValue),
    /// Frees the priority slot, written as NULL.
    Release,
    /// Writes another property than the present value, without a priority.
    WriteProperty {
        property: PropertyIdentifier,
        value: PropertyValue,
    },
    /// A command to the device, with what the log says about it. The password is sent, never stored.
    Command {
        text: String,
        password: Option<String>,
        kind: Command,
    },
    /// Done elsewhere, only logged here: acknowledging an alarm.
    Noted(String),
    /// Sets the device's clock (TimeSynchronization); `object` is the device itself.
    SetClock {
        date: Date,
        time: Time,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Restart(Restart),
    Communication {
        enable: bool,
        minutes: Option<u16>,
    },
    /// Read back from the log: done long ago, nothing to send.
    Logged,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub time: SystemTime,
    pub device: u32,
    /// Where the device was reached, to find it again after a restart; `None` behind a router.
    pub address: Option<Address>,
    pub object: ObjectIdentifier,
    /// `None` for objects without priority array.
    pub priority: Option<u8>,
    pub action: Action,
    /// Why the device refused, if it did.
    pub error: Option<String>,
}

impl Entry {
    /// What this entry set out of service to, if it did.
    fn out_of_service(&self) -> Option<bool> {
        match &self.action {
            Action::WriteProperty {
                property: PropertyIdentifier::OUT_OF_SERVICE,
                value: PropertyValue::Boolean(on),
            } => Some(*on),
            _ => None,
        }
    }

    fn same_slot(&self, other: &Entry) -> bool {
        self.device == other.device
            && self.object == other.object
            && self.priority == other.priority
    }
}

/// How long after a write the device's priority array may still show the old value.
const SETTLE_TIME: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug, Default)]
pub struct Journal {
    entries: Vec<Entry>,
    /// Writes another system has since written over or released, by position in `entries`: no
    /// longer ours, so releasing them would take away someone else's value.
    written_over: HashSet<usize>,
}

impl Journal {
    pub fn record(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Successful writes at a priority not released since, and objects put out of service and not
    /// taken back.
    pub fn held(&self) -> Vec<&Entry> {
        self.held_positions()
            .into_iter()
            .map(|position| &self.entries[position])
            .collect()
    }

    /// Compares the writes held on an object with its priority array as read now: a slot that
    /// holds something else, or nothing, was taken over by another system.
    pub fn compare_with_device(
        &mut self,
        device: u32,
        object: ObjectIdentifier,
        slots: &[Option<PropertyValue>],
    ) {
        let taken_over: Vec<usize> = self
            .held_positions()
            .into_iter()
            .filter(|&position| {
                let entry = &self.entries[position];
                let (Action::Write(written), Some(priority)) = (&entry.action, entry.priority)
                else {
                    return false;
                };
                // a device may show a write a moment late; only a settled one counts as taken over
                let settled = entry.time.elapsed().is_ok_and(|age| age >= SETTLE_TIME);
                settled
                    && entry.device == device
                    && entry.object == object
                    && slots.get(usize::from(priority) - 1) != Some(&Some(written.clone()))
            })
            .collect();
        self.written_over.extend(taken_over);
    }

    fn held_positions(&self) -> Vec<usize> {
        let mut held: Vec<usize> = Vec::new();
        for (position, entry) in self.entries.iter().enumerate() {
            if entry.error.is_some() {
                continue;
            }
            let entries = &self.entries;
            match &entry.action {
                Action::Write(_) | Action::Release => {
                    held.retain(|&earlier| {
                        let earlier = &entries[earlier];
                        earlier.out_of_service().is_some() || !earlier.same_slot(entry)
                    });
                    if entry.priority.is_some()
                        && matches!(entry.action, Action::Write(_))
                        && !self.written_over.contains(&position)
                    {
                        held.push(position);
                    }
                }
                Action::WriteProperty { .. } => {
                    // out of service also stays in the plant until someone takes it back
                    if let Some(on) = entry.out_of_service() {
                        held.retain(|&earlier| {
                            let earlier = &entries[earlier];
                            earlier.out_of_service().is_none() || !earlier.same_slot(entry)
                        });
                        if on {
                            held.push(position);
                        }
                    }
                }
                _ => {}
            }
        }
        held
    }
}

/// Performs the action: on the present value, or on the device's clock.
pub async fn perform(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
    priority: Option<u8>,
    action: &Action,
) -> Result<(), Error> {
    let (property, value) = match action {
        Action::Write(value) => (PropertyIdentifier::PRESENT_VALUE, value),
        Action::Release => (PropertyIdentifier::PRESENT_VALUE, &PropertyValue::Null),
        Action::WriteProperty { property, value } => (*property, value),
        Action::SetClock { date, time } => return set_clock(client, device, *date, *time).await,
        Action::Noted(_) => return Ok(()),
        Action::Command { password, kind, .. } => {
            return match kind {
                Command::Restart(restart) => {
                    commands::restart(client, device, *restart, password.clone()).await
                }
                Command::Communication { enable, minutes } => {
                    commands::communication(client, device, *enable, *minutes, password.clone())
                        .await
                }
                Command::Logged => Ok(()),
            };
        }
    };
    let mut encoded = BytesMut::new();
    encode_property_value(&mut encoded, value)?;
    client
        .write_property_to_device(device, object, property, None, encoded.to_vec(), priority)
        .await
}

/// Properties that are not written as plain values, or that have their own way here: the present
/// value goes through a priority, the object's identity must not change by a typing slip.
pub const NOT_WRITABLE: [PropertyIdentifier; 5] = [
    PropertyIdentifier::PRESENT_VALUE,
    PropertyIdentifier::OBJECT_IDENTIFIER,
    PropertyIdentifier::OBJECT_TYPE,
    PropertyIdentifier::OBJECT_LIST,
    PropertyIdentifier::PROPERTY_LIST,
];

/// A typed value from what was typed, of the kind the property holds now (`data_type` in the API).
pub fn parse_property_value(kind: &str, text: &str) -> Result<PropertyValue, String> {
    let text = text.trim();
    let invalid = |what: &str| format!("\"{text}\" is not {what}");
    match kind {
        "boolean" => match text {
            "true" | "1" => Ok(PropertyValue::Boolean(true)),
            "false" | "0" => Ok(PropertyValue::Boolean(false)),
            _ => Err(invalid("true or false")),
        },
        "text" => Ok(PropertyValue::CharacterString(text.to_string())),
        // "NaN" and "inf" parse as numbers, but no limit or increment should ever be one
        "real" => text
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|value: &f32| value.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(|| invalid("a number")),
        "double" => text
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|value: &f64| value.is_finite())
            .map(PropertyValue::Double)
            .ok_or_else(|| invalid("a number")),
        "unsigned" => text
            .parse()
            .map(PropertyValue::Unsigned)
            .map_err(|_| invalid("a whole number")),
        "signed" => text
            .parse()
            .map(PropertyValue::Signed)
            .map_err(|_| invalid("a whole number")),
        "enumerated" => text
            .parse()
            .map(PropertyValue::Enumerated)
            .map_err(|_| invalid("the number of a choice")),
        other => Err(format!("a value of kind {other} cannot be written here")),
    }
}

/// Sets the device's clock to the given local date and time (TimeSynchronization). The service is
/// unconfirmed: success means it was sent, not that the device took it.
async fn set_clock(client: &Client, device: u32, date: Date, time: Time) -> Result<(), Error> {
    client
        .time_synchronization_from_device(device, date, time)
        .await
}

/// "2026-11-01" and "09:30:05" with the weekday (1 = Monday) to BACnet date and time.
pub fn parse_clock(date: &str, time: &str, weekday: u8) -> Result<(Date, Time), String> {
    let invalid = || format!("{date} {time} is not a date and time like 2026-11-01 09:30:05");
    let numbers = |text: &str, separator: char| -> Result<Vec<u32>, String> {
        text.split(separator)
            .map(|part| part.trim().parse().map_err(|_| invalid()))
            .collect()
    };
    let (Ok([year, month, day]), Ok([hour, minute, second])) = (
        <[u32; 3]>::try_from(numbers(date, '-')?),
        <[u32; 3]>::try_from(numbers(time, ':')?),
    ) else {
        return Err(invalid());
    };
    let fits = (1900..=2154).contains(&year)
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && (1..=7).contains(&weekday)
        && hour < 24
        && minute < 60
        && second < 60;
    if !fits {
        return Err(invalid());
    }
    let date = Date {
        year: (year - 1900) as u8,
        month: month as u8,
        day: day as u8,
        day_of_week: weekday,
    };
    let time = Time {
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
        hundredths: 0,
    };
    Ok((date, time))
}

/// The value typed by the user, in the data type the object expects.
pub fn parse_value(object_type: ObjectType, text: &str) -> Result<PropertyValue, String> {
    let text = text.trim();
    match object_type {
        ObjectType::ANALOG_INPUT | ObjectType::ANALOG_OUTPUT | ObjectType::ANALOG_VALUE => text
            .replace(',', ".")
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(|| format!("\"{text}\" is not a number")),
        ObjectType::BINARY_INPUT | ObjectType::BINARY_OUTPUT | ObjectType::BINARY_VALUE => {
            match text.to_lowercase().as_str() {
                "active" | "on" | "1" | "true" => Ok(PropertyValue::Enumerated(1)),
                "inactive" | "off" | "0" | "false" => Ok(PropertyValue::Enumerated(0)),
                _ => Err(format!("\"{text}\" is not active or inactive")),
            }
        }
        ObjectType::MULTI_STATE_INPUT
        | ObjectType::MULTI_STATE_OUTPUT
        | ObjectType::MULTI_STATE_VALUE => text
            .parse::<u64>()
            .ok()
            .filter(|state| *state >= 1)
            .map(PropertyValue::Unsigned)
            .ok_or_else(|| format!("\"{text}\" is not a state number (1, 2, ...)")),
        ObjectType::LIGHTING_OUTPUT => text
            .replace(',', ".")
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(|| format!("\"{text}\" is not a number")),
        ObjectType::LARGE_ANALOG_VALUE => text
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(PropertyValue::Double)
            .ok_or_else(|| format!("\"{text}\" is not a number")),
        ObjectType::INTEGER_VALUE => text
            .parse::<i32>()
            .map(PropertyValue::Signed)
            .map_err(|_| format!("\"{text}\" is not a whole number")),
        ObjectType::POSITIVE_INTEGER_VALUE => text
            .parse::<u64>()
            .map(PropertyValue::Unsigned)
            .map_err(|_| format!("\"{text}\" is not a positive whole number")),
        ObjectType::CHARACTERSTRING_VALUE => Ok(PropertyValue::CharacterString(text.to_string())),
        ObjectType::BINARY_LIGHTING_OUTPUT => text
            .parse::<u32>()
            .ok()
            .filter(|state| *state <= 6)
            .map(PropertyValue::Enumerated)
            .ok_or_else(|| format!("\"{text}\" is not a lighting state (0 off, 1 on, ...)")),
        _ => Err("objects of this type cannot be written yet".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_other_commandable_types_take_a_value_of_their_kind() {
        let parsed = |object_type, text| parse_value(object_type, text);
        assert_eq!(
            parsed(ObjectType::INTEGER_VALUE, "-5"),
            Ok(PropertyValue::Signed(-5))
        );
        assert!(parsed(ObjectType::INTEGER_VALUE, "1.5").is_err());
        assert_eq!(
            parsed(ObjectType::POSITIVE_INTEGER_VALUE, "7"),
            Ok(PropertyValue::Unsigned(7))
        );
        assert!(parsed(ObjectType::POSITIVE_INTEGER_VALUE, "-1").is_err());
        assert_eq!(
            parsed(ObjectType::LARGE_ANALOG_VALUE, "1234567,5"),
            Ok(PropertyValue::Double(1_234_567.5))
        );
        assert_eq!(
            parsed(ObjectType::LIGHTING_OUTPUT, "80"),
            Ok(PropertyValue::Real(80.0))
        );
        assert_eq!(
            parsed(ObjectType::CHARACTERSTRING_VALUE, " Winter "),
            Ok(PropertyValue::CharacterString("Winter".to_string()))
        );
        assert_eq!(
            parsed(ObjectType::BINARY_LIGHTING_OUTPUT, "1"),
            Ok(PropertyValue::Enumerated(1))
        );
        assert!(parsed(ObjectType::BINARY_LIGHTING_OUTPUT, "9").is_err());
        assert!(parsed(ObjectType::DEVICE, "1").is_err());
    }

    fn entry(priority: Option<u8>, action: Action, error: Option<&str>) -> Entry {
        Entry {
            time: SystemTime::UNIX_EPOCH,
            device: 1,
            address: None,
            object: ObjectIdentifier::new(ObjectType::ANALOG_VALUE, 1).unwrap(),
            priority,
            action,
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn a_write_another_system_has_written_over_is_no_longer_held() {
        let mut journal = Journal::default();
        journal.record(entry(
            Some(8),
            Action::Write(PropertyValue::Real(30.0)),
            None,
        ));
        let object = journal.entries()[0].object;
        let mut slots = vec![None; 16];
        slots[7] = Some(PropertyValue::Real(30.0));
        journal.compare_with_device(1, object, &slots);
        assert_eq!(journal.held().len(), 1, "still what we wrote");
        slots[7] = Some(PropertyValue::Real(100.0));
        journal.compare_with_device(1, object, &slots);
        assert!(journal.held().is_empty(), "the plant wrote 100 over our 30");
        journal.record(entry(
            Some(8),
            Action::Write(PropertyValue::Real(50.0)),
            None,
        ));
        assert_eq!(journal.held().len(), 1, "a new write is held again");
    }

    #[test]
    fn values_are_typed_by_object() {
        assert_eq!(
            parse_value(ObjectType::ANALOG_VALUE, " 21,5 "),
            Ok(PropertyValue::Real(21.5))
        );
        assert_eq!(
            parse_value(ObjectType::BINARY_OUTPUT, "Active"),
            Ok(PropertyValue::Enumerated(1))
        );
        assert_eq!(
            parse_value(ObjectType::MULTI_STATE_VALUE, "2"),
            Ok(PropertyValue::Unsigned(2))
        );
        assert!(parse_value(ObjectType::MULTI_STATE_VALUE, "0").is_err());
        assert!(parse_value(ObjectType::ANALOG_VALUE, "warm").is_err());
        assert!(parse_value(ObjectType::DEVICE, "1").is_err());
    }

    #[test]
    fn held_values_follow_the_journal() {
        let mut journal = Journal::default();
        journal.record(entry(
            Some(8),
            Action::Write(PropertyValue::Real(1.0)),
            None,
        ));
        journal.record(entry(
            Some(8),
            Action::Write(PropertyValue::Real(2.0)),
            None,
        ));
        journal.record(entry(
            Some(9),
            Action::Write(PropertyValue::Real(3.0)),
            Some("refused"),
        ));
        journal.record(entry(None, Action::Write(PropertyValue::Real(4.0)), None));
        let held = journal.held();
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].action, Action::Write(PropertyValue::Real(2.0)));

        journal.record(entry(Some(8), Action::Release, None));
        assert!(journal.held().is_empty());
        assert_eq!(journal.entries().len(), 5);
    }

    #[test]
    fn clock_from_text() {
        let (date, time) = parse_clock("2026-11-01", "09:30:05", 7).unwrap();
        assert_eq!(
            (date.year, date.month, date.day, date.day_of_week),
            (126, 11, 1, 7)
        );
        assert_eq!((time.hour, time.minute, time.second), (9, 30, 5));
        assert!(parse_clock("2026-13-01", "09:30:05", 7).is_err());
        assert!(parse_clock("2026-11-01", "9:30", 7).is_err());
    }
}
