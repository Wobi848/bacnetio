//! What the API sends: the library's results in plain words, ready to show (docs/api.md).

use std::time::UNIX_EPOCH;

use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use std::collections::BTreeMap;

use serde::Serialize;

use crate::interfaces::{Adapter, AdapterKind};
use crate::object::ObjectDetail;
use crate::points::Point;
use crate::present;

use crate::scan::{Device, Phase};
use crate::write::Entry;

/// A note with its code and values for the UI to translate, and the English text as fallback.
#[derive(Serialize, Clone)]
pub struct NoteView<T> {
    #[serde(flatten)]
    pub note: T,
    pub text: String,
}

impl<T: std::fmt::Display> NoteView<T> {
    pub fn new(note: T) -> Self {
        NoteView {
            text: note.to_string(),
            note,
        }
    }
}

#[derive(Serialize)]
pub struct AdapterView {
    pub name: String,
    pub ip: String,
    pub prefix_len: u8,
    pub kind: &'static str,
    pub default: bool,
    /// Only where the system has one apart from the name (Windows).
    pub description: Option<String>,
}

impl AdapterView {
    pub fn new(adapter: &Adapter, default: bool) -> Self {
        AdapterView {
            name: adapter.name.clone(),
            ip: adapter.ip.to_string(),
            prefix_len: adapter.prefix_len,
            kind: match adapter.kind() {
                AdapterKind::Cable => "cable",
                AdapterKind::Wireless => "wireless",
                AdapterKind::Virtual => "virtual",
            },
            default,
            description: adapter.description.clone(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct DeviceView {
    pub instance: u32,
    pub address: String,
    /// Network number and MAC (hex) for a device behind a BACnet router.
    pub network: Option<u16>,
    pub mac: Option<String>,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub found_by: &'static str,
    pub error: Option<String>,
    pub objects: ObjectsView,
}

/// How far the search has read the device's object list.
#[derive(Serialize, Clone)]
pub struct ObjectsView {
    pub read: usize,
    pub total: Option<usize>,
    pub done: bool,
    pub error: Option<String>,
    /// How many objects of each type, by type number.
    pub types: BTreeMap<u32, usize>,
}

impl DeviceView {
    pub fn new(device: &Device) -> Self {
        DeviceView {
            instance: device.instance,
            address: present::address_text(device.address),
            network: device.remote.as_ref().map(|remote| remote.network),
            mac: device
                .remote
                .as_ref()
                .map(|remote| present::hex(&remote.mac)),
            name: device.details.name.clone(),
            vendor: device.details.vendor.clone(),
            model: device.details.model.clone(),
            found_by: if device.answered_who_is {
                "who_is"
            } else {
                "direct"
            },
            error: device.details.error.clone(),
            objects: ObjectsView {
                read: device.objects.read,
                total: device.objects.total,
                done: device.objects.objects.is_some(),
                error: device.objects.error.clone(),
                types: device.objects.objects.iter().flatten().fold(
                    BTreeMap::new(),
                    |mut types, object| {
                        *types.entry(object.object_type().to_raw()).or_default() += 1;
                        types
                    },
                ),
            },
        }
    }
}

pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::WhoIs => "who_is",
        Phase::Sweep { .. } => "sweep",
        Phase::Details => "details",
        Phase::ObjectLists => "objects",
    }
}

#[derive(Serialize, Clone)]
pub struct ObjectRef {
    #[serde(rename = "type")]
    pub object_type: u32,
    pub instance: u32,
    /// "Analog input 3"
    pub object: String,
}

impl ObjectRef {
    pub fn new(object: ObjectIdentifier) -> Self {
        ObjectRef {
            object_type: object.object_type().to_raw(),
            instance: object.instance_number(),
            object: present::object_text(object),
        }
    }
}

#[derive(Serialize)]
pub struct PointView {
    #[serde(flatten)]
    pub object: ObjectRef,
    pub name: Option<String>,
    pub description: Option<String>,
    /// With unit: "21.4 °C".
    pub value: String,
    pub units: Option<String>,
    /// Status flags that are set, in words; empty when all is well.
    pub status: String,
    /// Texts of a multi-state object's states, from state 1 on; `value` already uses them.
    pub state_texts: Vec<String>,
    /// A binary object's inactive and active text, in this order; empty when it has none.
    pub binary_texts: Vec<String>,
    /// The notification class of an object that reports alarms by itself.
    pub notification_class: Option<u32>,
    pub error: Option<String>,
}

impl PointView {
    pub fn new(point: &Point) -> Self {
        PointView {
            object: ObjectRef::new(point.object),
            name: point.name.clone(),
            description: point.description.clone(),
            value: present::shown_value(point),
            units: point.units.map(present::unit_text),
            status: present::status_text(point.status),
            state_texts: point.state_texts.clone(),
            binary_texts: point.binary_texts.clone(),
            notification_class: point.notification_class,
            error: point.error.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct SlotView {
    pub priority: u8,
    /// `None` when the slot is free.
    pub value: Option<String>,
    /// Written by bacnetio and not released yet.
    pub mine: bool,
}

#[derive(Serialize)]
pub struct ObjectView {
    #[serde(flatten)]
    pub point: PointView,
    pub commandable: bool,
    pub priority_array: Option<Vec<SlotView>>,
    pub relinquish_default: Option<String>,
}

impl ObjectView {
    /// `mine` lists the priorities bacnetio wrote on this object, with what it wrote: a slot
    /// another system has written over since is no longer ours.
    pub fn new(detail: &ObjectDetail, mine: &[(u8, PropertyValue)]) -> Self {
        let text = |value: &PropertyValue| {
            present::value_with_unit(detail.point.object.object_type(), value, detail.point.units)
        };
        let priority_array = detail.priority_array.as_ref().map(|slots| {
            (1..)
                .zip(slots)
                .map(|(priority, slot)| SlotView {
                    priority,
                    value: slot.as_ref().map(text),
                    mine: mine
                        .iter()
                        .any(|(held, written)| *held == priority && slot.as_ref() == Some(written)),
                })
                .collect()
        });
        ObjectView {
            point: PointView::new(&detail.point),
            commandable: detail.commandable(),
            priority_array,
            relinquish_default: detail.relinquish_default.as_ref().map(text),
        }
    }
}

#[derive(Serialize)]
pub struct EntryView {
    /// Seconds since 1970-01-01 UTC.
    pub time: u64,
    pub device: u32,
    #[serde(flatten)]
    pub object: ObjectRef,
    pub priority: Option<u8>,
    pub action: &'static str,
    /// The value written, in words; `None` for a release.
    pub value: Option<String>,
    pub error: Option<String>,
    /// Where the device was reached, for people: `10.0.0.20`, `MS/TP 5`; `None` behind a router.
    pub address: Option<String>,
    /// The entry stopped the device's communication without end: held until it talks again.
    pub stops_communication: bool,
    /// The stop covers only what the device sends by itself; it still answers.
    pub sending_only: bool,
}

impl EntryView {
    pub fn new(entry: &Entry) -> Self {
        let (action, value) = present::entry_action(entry);
        EntryView {
            time: entry
                .time
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
            device: entry.device,
            object: ObjectRef::new(entry.object),
            priority: entry.priority,
            action,
            value,
            error: entry.error.clone(),
            address: entry.address.map(|address| address.shown()),
            stops_communication: entry.is_communication_stop(),
            sending_only: entry.stops_sending_only(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use bacnet_types::enums::{ObjectType, PropertyIdentifier};
    use bacnet_types::primitives::{Date, Time};

    use super::*;
    use crate::commands::Restart;
    use crate::write::{Action, Command};

    /// Every action the log can carry is in docs/api.md: a page that shows the log switches on it,
    /// and one it does not know breaks that page. One entry per variant of `Action`, so a variant
    /// added to the exhaustive match in `present::entry_action` belongs in this list too.
    #[test]
    fn every_action_of_the_log_is_documented() {
        let object = ObjectIdentifier::new(ObjectType::ANALOG_OUTPUT, 1).expect("valid object");
        let entry = |action| Entry {
            time: SystemTime::UNIX_EPOCH,
            device: 1001,
            address: None,
            object,
            priority: None,
            action,
            error: None,
        };
        let time = Time {
            hour: 9,
            minute: 30,
            second: 5,
            hundredths: 0,
        };
        let actions = [
            Action::Write(PropertyValue::Real(42.5)),
            Action::Release,
            Action::WriteProperty {
                property: PropertyIdentifier::OUT_OF_SERVICE,
                value: PropertyValue::Boolean(true),
            },
            Action::Command {
                text: "warm start".into(),
                password: None,
                kind: Command::Restart(Restart::Warm),
            },
            Action::Noted("Analog output 1 acknowledged".into()),
            Action::SetClock {
                date: Date {
                    year: 126,
                    month: 11,
                    day: 2,
                    day_of_week: 1,
                },
                time,
            },
        ];
        let doc = std::fs::read_to_string("docs/api.md").expect("api.md is readable");
        for action in actions {
            let view = EntryView::new(&entry(action));
            assert!(
                doc.contains(&format!("`{}`", view.action)),
                "docs/api.md does not name the action {}",
                view.action
            );
        }
    }
}
