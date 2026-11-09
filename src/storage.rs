//! The write log on disk, so values still held survive a crash or restart (concept section 6):
//! one JSON object per line, appended after every write and release.

use crate::address::Address;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use serde::{Deserialize, Serialize};

use crate::write::{Action, Command, Entry, parse_clock};

pub const JOURNAL_FILE: &str = "writes.jsonl";

#[derive(Serialize, Deserialize)]
struct StoredEntry {
    time_ms: u64,
    device: u32,
    /// Missing in logs written before it was stored.
    #[serde(default)]
    address: Option<Address>,
    object_type: u32,
    instance: u32,
    priority: Option<u8>,
    action: StoredAction,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StoredAction {
    Write {
        value: StoredValue,
    },
    Release,
    WriteProperty {
        property: u32,
        value: StoredValue,
    },
    /// What a command did, in words; never its password.
    Command {
        text: String,
        /// A stop or start of the device's communication, so a stop without end is still held
        /// after a restart. Missing in logs written before, and for other commands.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        communication: Option<StoredCommunication>,
    },
    /// "2026-11-01", "09:30:05", weekday 1 = Monday.
    SetClock {
        date: String,
        time: String,
        weekday: u8,
    },
}

#[derive(Serialize, Deserialize)]
struct StoredCommunication {
    enable: bool,
    /// Missing in logs written before both kinds of stop existed: those stopped all of it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    sending_only: bool,
    minutes: Option<u16>,
}

/// The value types bacnetio writes.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum StoredValue {
    Real(f32),
    Double(f64),
    Unsigned(u64),
    Signed(i32),
    Enumerated(u32),
    Boolean(bool),
    Text(String),
    /// Object type and instance.
    Object(u32, u32),
}

fn stored_value(value: &PropertyValue) -> Option<StoredValue> {
    Some(match value {
        PropertyValue::Real(value) => StoredValue::Real(*value),
        PropertyValue::Double(value) => StoredValue::Double(*value),
        PropertyValue::Unsigned(value) => StoredValue::Unsigned(*value),
        PropertyValue::Signed(value) => StoredValue::Signed(*value),
        PropertyValue::Enumerated(value) => StoredValue::Enumerated(*value),
        PropertyValue::Boolean(value) => StoredValue::Boolean(*value),
        PropertyValue::CharacterString(value) => StoredValue::Text(value.clone()),
        PropertyValue::ObjectIdentifier(object) => {
            StoredValue::Object(object.object_type().to_raw(), object.instance_number())
        }
        _ => return None,
    })
}

fn property_value(value: StoredValue) -> Option<PropertyValue> {
    Some(match value {
        StoredValue::Real(value) => PropertyValue::Real(value),
        StoredValue::Double(value) => PropertyValue::Double(value),
        StoredValue::Unsigned(value) => PropertyValue::Unsigned(value),
        StoredValue::Signed(value) => PropertyValue::Signed(value),
        StoredValue::Enumerated(value) => PropertyValue::Enumerated(value),
        StoredValue::Boolean(value) => PropertyValue::Boolean(value),
        StoredValue::Text(value) => PropertyValue::CharacterString(value),
        StoredValue::Object(object_type, instance) => PropertyValue::ObjectIdentifier(
            ObjectIdentifier::new(ObjectType::from_raw(object_type), instance).ok()?,
        ),
    })
}

/// Where bacnetio keeps its files when none is given: the user's local application data.
pub fn default_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("bacnetio"));
    }
    if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(state).join("bacnetio"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state/bacnetio"))
}

pub fn append(path: &Path, entry: &Entry) -> io::Result<()> {
    let stored = stored(entry).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "value type cannot be stored")
    })?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut line = serde_json::to_string(&stored).map_err(io::Error::other)?;
    line.push('\n');
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}

/// All entries in the file, oldest first, and how many lines could not be read. A missing file
/// is an empty log.
pub fn load(path: &Path) -> io::Result<(Vec<Entry>, usize)> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok((Vec::new(), 0)),
        Err(error) => return Err(error),
    };
    let mut entries = Vec::new();
    let mut unreadable = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<StoredEntry>(line)
            .ok()
            .and_then(entry)
        {
            Some(found) => entries.push(found),
            None => unreadable += 1,
        }
    }
    Ok((entries, unreadable))
}

fn stored(entry: &Entry) -> Option<StoredEntry> {
    let action = match &entry.action {
        Action::Release => StoredAction::Release,
        Action::Write(value) => StoredAction::Write {
            value: stored_value(value)?,
        },
        Action::WriteProperty { property, value } => StoredAction::WriteProperty {
            property: property.to_raw(),
            value: stored_value(value)?,
        },
        Action::Command { text, kind, .. } => StoredAction::Command {
            text: text.clone(),
            communication: match kind {
                Command::Communication {
                    enable,
                    sending_only,
                    minutes,
                } => Some(StoredCommunication {
                    enable: *enable,
                    sending_only: *sending_only,
                    minutes: *minutes,
                }),
                _ => None,
            },
        },
        Action::Noted(text) => StoredAction::Command {
            text: text.clone(),
            communication: None,
        },
        Action::SetClock { date, time } => StoredAction::SetClock {
            date: format!(
                "{}-{:02}-{:02}",
                u32::from(date.year) + 1900,
                date.month,
                date.day
            ),
            time: format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second),
            weekday: date.day_of_week,
        },
    };
    Some(StoredEntry {
        time_ms: entry
            .time
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as u64),
        device: entry.device,
        address: entry.address,
        object_type: entry.object.object_type().to_raw(),
        instance: entry.object.instance_number(),
        priority: entry.priority,
        action,
        error: entry.error.clone(),
    })
}

fn entry(stored: StoredEntry) -> Option<Entry> {
    let action = match stored.action {
        StoredAction::Release => Action::Release,
        StoredAction::Write { value } => Action::Write(property_value(value)?),
        StoredAction::WriteProperty { property, value } => Action::WriteProperty {
            property: PropertyIdentifier::from_raw(property),
            value: property_value(value)?,
        },
        StoredAction::Command {
            text,
            communication:
                Some(StoredCommunication {
                    enable,
                    sending_only,
                    minutes,
                }),
        } => Action::Command {
            text,
            password: None,
            kind: Command::Communication {
                enable,
                sending_only,
                minutes,
            },
        },
        StoredAction::Command { text, .. } => Action::Noted(text),
        StoredAction::SetClock {
            date,
            time,
            weekday,
        } => {
            let (date, time) = parse_clock(&date, &time, weekday).ok()?;
            Action::SetClock { date, time }
        }
    };
    Some(Entry {
        time: UNIX_EPOCH + Duration::from_millis(stored.time_ms),
        device: stored.device,
        address: stored.address,
        object: ObjectIdentifier::new(ObjectType::from_raw(stored.object_type), stored.instance)
            .ok()?,
        priority: stored.priority,
        action,
        error: stored.error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::Journal;

    fn temporary(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bacnetio-{name}-{}", std::process::id()));
        // left over from an aborted run, or not there at all
        let _ = fs::remove_file(&path);
        path
    }

    fn write(priority: u8, value: PropertyValue) -> Entry {
        Entry {
            time: UNIX_EPOCH + Duration::from_millis(1_793_534_400_123),
            device: 3001,
            address: Some("10.0.0.20:47808".parse().unwrap()),
            object: ObjectIdentifier::new(ObjectType::ANALOG_OUTPUT, 1).unwrap(),
            priority: Some(priority),
            action: Action::Write(value),
            error: None,
        }
    }

    #[test]
    fn held_values_survive_a_restart() {
        let path = temporary("journal");
        append(&path, &write(8, PropertyValue::Real(21.4))).unwrap();
        append(&path, &write(9, PropertyValue::Enumerated(1))).unwrap();
        let release = Entry {
            action: Action::Release,
            ..write(9, PropertyValue::Null)
        };
        append(&path, &release).unwrap();

        let (entries, unreadable) = load(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(unreadable, 0);
        assert_eq!(entries[0], write(8, PropertyValue::Real(21.4)));
        let mut journal = Journal::default();
        entries.into_iter().for_each(|entry| journal.record(entry));
        assert_eq!(journal.held().len(), 1);
        assert_eq!(journal.held()[0].priority, Some(8));
    }

    #[test]
    fn a_clock_setting_is_logged_but_not_held() {
        let path = temporary("clock");
        let (date, time) = parse_clock("2026-11-01", "09:30:05", 7).unwrap();
        let clock = Entry {
            object: ObjectIdentifier::new(ObjectType::DEVICE, 3001).unwrap(),
            priority: None,
            action: Action::SetClock { date, time },
            ..write(8, PropertyValue::Null)
        };
        append(&path, &clock).unwrap();
        let (entries, unreadable) = load(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!((entries.clone(), unreadable), (vec![clock], 0));
        let mut journal = Journal::default();
        entries.into_iter().for_each(|entry| journal.record(entry));
        assert!(journal.held().is_empty());
    }

    #[test]
    fn a_communication_stop_without_end_is_still_held_after_a_restart() {
        let path = temporary("communication");
        let stop = Entry {
            object: ObjectIdentifier::new(ObjectType::DEVICE, 3001).unwrap(),
            priority: None,
            action: Action::Command {
                text: "communication off".to_string(),
                password: Some("secret-word".to_string()),
                kind: Command::Communication {
                    enable: false,
                    sending_only: true,
                    minutes: None,
                },
            },
            ..write(8, PropertyValue::Null)
        };
        append(&path, &stop).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("secret-word"),
            "the password stays out of the file"
        );
        let (entries, unreadable) = load(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(unreadable, 0);
        let mut journal = Journal::default();
        entries.into_iter().for_each(|entry| journal.record(entry));
        assert!(journal.held()[0].is_communication_stop());
        assert!(
            matches!(
                journal.held()[0].action,
                Action::Command {
                    kind: Command::Communication {
                        sending_only: true,
                        ..
                    },
                    ..
                }
            ),
            "the kind of stop comes back too"
        );
    }

    #[test]
    fn commands_of_logs_written_before_load_as_words() {
        let path = temporary("old-command");
        fs::write(
            &path,
            r#"{"time_ms":1,"device":7,"object_type":8,"instance":7,"priority":null,"action":{"kind":"command","text":"communication off"},"error":null}"#,
        )
        .unwrap();
        let (entries, unreadable) = load(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(unreadable, 0);
        assert_eq!(
            entries[0].action,
            Action::Noted("communication off".to_string())
        );
    }

    #[test]
    fn a_broken_line_is_skipped_and_counted() {
        let path = temporary("broken");
        append(&path, &write(8, PropertyValue::Real(1.0))).unwrap();
        fs::write(
            &path,
            format!("{}not json\n", fs::read_to_string(&path).unwrap()),
        )
        .unwrap();
        let (entries, unreadable) = load(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!((entries.len(), unreadable), (1, 1));
    }

    #[test]
    fn logs_without_address_still_load() {
        let path = temporary("old");
        fs::write(
            &path,
            r#"{"time_ms":1,"device":7,"object_type":1,"instance":1,"priority":8,"action":{"kind":"release"},"error":null}"#,
        )
        .unwrap();
        let (entries, unreadable) = load(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!((entries.len(), unreadable), (1, 0));
        assert_eq!(entries[0].address, None);
    }

    #[test]
    fn a_missing_file_is_an_empty_log() {
        assert_eq!(load(&temporary("missing")).unwrap().0, Vec::new());
    }
}
