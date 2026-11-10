//! Writing present values and releasing them again (concept section 6). Every write lands in the
//! journal; the values still held are derived from it, so nothing is left behind when leaving.

use crate::address::Address;
use std::collections::HashSet;
use std::time::SystemTime;

use bacnet_encoding::primitives::encode_property_value;
use bacnet_types::enums::PropertyIdentifier;
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

impl Action {
    /// Lets a device talk again: it does not answer before, so it is not asked where it is.
    pub fn lets_talk_again(&self) -> bool {
        matches!(
            self,
            Action::Command {
                kind: Command::Communication { enable: true, .. },
                ..
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Restart(Restart),
    Communication {
        enable: bool,
        /// Stops only what the device sends by itself (DISABLE_INITIATION); it still answers.
        sending_only: bool,
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

    /// Whether this entry stopped the device's communication for good (`Some(true)`) or let it
    /// talk again (`Some(false)`); a stop for some minutes ends by itself and holds nothing.
    fn stops_communication(&self) -> Option<bool> {
        match &self.action {
            Action::Command {
                kind:
                    Command::Communication {
                        enable, minutes, ..
                    },
                ..
            } => Some(!enable && minutes.is_none()),
            _ => None,
        }
    }

    /// The device's communication stopped for good by this entry and not let on since.
    pub fn is_communication_stop(&self) -> bool {
        self.stops_communication() == Some(true)
    }

    /// A stop of only what the device sends by itself: it still answers.
    pub fn stops_sending_only(&self) -> bool {
        matches!(
            self.action,
            Action::Command {
                kind: Command::Communication {
                    enable: false,
                    sending_only: true,
                    ..
                },
                ..
            }
        )
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

    /// Successful writes at a priority not released since, objects put out of service and not
    /// taken back, and devices whose communication was stopped for good and not let on again.
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
                Action::Command { .. } => {
                    // a stop without end stays in the plant like a value held
                    if let Some(stops) = entry.stops_communication() {
                        held.retain(|&earlier| {
                            let earlier = &entries[earlier];
                            earlier.stops_communication().is_none()
                                || earlier.device != entry.device
                        });
                        if stops {
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
                Command::Communication {
                    enable,
                    sending_only,
                    minutes,
                } => {
                    commands::communication(
                        client,
                        device,
                        commands::communication_state(*enable, *sending_only),
                        *minutes,
                        password.clone(),
                    )
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

/// Sets the device's clock to the given local date and time (TimeSynchronization). The service is
/// unconfirmed: success means it was sent, not that the device took it.
async fn set_clock(client: &Client, device: u32, date: Date, time: Time) -> Result<(), Error> {
    client
        .time_synchronization_from_device(device, date, time)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::enums::ObjectType;

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

    fn communication(enable: bool, minutes: Option<u16>) -> Action {
        Action::Command {
            text: String::new(),
            password: None,
            kind: Command::Communication {
                enable,
                sending_only: false,
                minutes,
            },
        }
    }

    #[test]
    fn a_communication_stop_without_end_is_held_until_the_device_talks_again() {
        let mut journal = Journal::default();
        // a stop for some minutes ends by itself
        journal.record(entry(None, communication(false, Some(30)), None));
        assert!(journal.held().is_empty());
        // a refused stop holds nothing
        journal.record(entry(
            None,
            communication(false, None),
            Some("password-failure"),
        ));
        assert!(journal.held().is_empty());
        journal.record(entry(None, communication(false, None), None));
        assert_eq!(journal.held().len(), 1);
        assert!(journal.held()[0].is_communication_stop());
        // another device's start does not count
        journal.record(Entry {
            device: 2,
            ..entry(None, communication(true, None), None)
        });
        assert_eq!(journal.held().len(), 1);
        journal.record(entry(None, communication(true, None), None));
        assert!(journal.held().is_empty());
    }
}
