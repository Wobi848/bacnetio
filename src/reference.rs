//! A device's points saved as reference, and what changed since: before and after a
//! commissioning, or between two visits (concept milestone M6, compare snapshots).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::points::Point;
use crate::present;

const FOLDER: &str = "references";
/// Object types whose values drift by themselves; their changes are shown but not counted.
const ANALOG_TYPES: [u32; 3] = [0, 1, 2];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedPoint {
    pub object_type: u32,
    pub instance: u32,
    pub name: Option<String>,
    /// As shown: "21.4 °C", "active".
    pub value: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    pub device: u32,
    pub time_ms: u64,
    pub points: Vec<SavedPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Difference {
    Value { was: String },
    Status { was: String },
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    pub object_type: u32,
    pub instance: u32,
    pub name: Option<String>,
    #[serde(flatten)]
    pub difference: Difference,
    /// An analog value that drifts by itself; not counted as a change.
    pub drift: bool,
}

pub fn from_points(device: u32, time: SystemTime, points: &[Point]) -> Reference {
    Reference {
        device,
        time_ms: crate::unix_time::millis(time),
        points: points.iter().map(saved).collect(),
    }
}

fn saved(point: &Point) -> SavedPoint {
    SavedPoint {
        object_type: point.object.object_type().to_raw(),
        instance: point.object.instance_number(),
        name: point.name.clone(),
        value: present::value_text(point),
        status: present::status_text(point.status),
    }
}

/// What differs between the reference and the points read now.
pub fn compare(reference: &Reference, now: &[Point]) -> Vec<Change> {
    let current: Vec<SavedPoint> = now.iter().map(saved).collect();
    let same_object =
        |a: &SavedPoint, b: &SavedPoint| a.object_type == b.object_type && a.instance == b.instance;
    let change = |point: &SavedPoint, difference: Difference| Change {
        object_type: point.object_type,
        instance: point.instance,
        name: point.name.clone(),
        drift: matches!(difference, Difference::Value { .. })
            && ANALOG_TYPES.contains(&point.object_type),
        difference,
    };
    let mut changes = Vec::new();
    for point in &current {
        match reference
            .points
            .iter()
            .find(|before| same_object(before, point))
        {
            None => changes.push(change(point, Difference::Added)),
            Some(before) => {
                if before.value != point.value {
                    let was = before.value.clone();
                    changes.push(change(point, Difference::Value { was }));
                }
                if before.status != point.status {
                    let was = before.status.clone();
                    changes.push(change(point, Difference::Status { was }));
                }
            }
        }
    }
    for before in &reference.points {
        if !current.iter().any(|point| same_object(point, before)) {
            changes.push(change(before, Difference::Removed));
        }
    }
    changes
}

fn path(dir: &Path, device: u32) -> PathBuf {
    dir.join(FOLDER).join(format!("device-{device}.json"))
}

pub fn save(dir: &Path, reference: &Reference) -> io::Result<()> {
    let path = path(dir, reference.device);
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let json = serde_json::to_string_pretty(reference).map_err(io::Error::other)?;
    fs::write(path, json)
}

/// The saved reference of the device, if there is one.
pub fn load(dir: &Path, device: u32) -> io::Result<Option<Reference>> {
    match fs::read_to_string(path(dir, device)) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::enums::ObjectType;
    use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
    use std::time::UNIX_EPOCH;

    fn point(object_type: ObjectType, instance: u32, value: PropertyValue) -> Point {
        let mut point = Point::new(ObjectIdentifier::new(object_type, instance).unwrap());
        point.value = Some(value);
        point
    }

    #[test]
    fn changes_states_and_objects_drift_only_flagged() {
        let before = vec![
            point(ObjectType::ANALOG_INPUT, 1, PropertyValue::Real(21.4)),
            point(ObjectType::BINARY_OUTPUT, 1, PropertyValue::Enumerated(0)),
            point(ObjectType::BINARY_INPUT, 9, PropertyValue::Enumerated(0)),
        ];
        let reference = from_points(7, UNIX_EPOCH, &before);
        let now = vec![
            point(ObjectType::ANALOG_INPUT, 1, PropertyValue::Real(21.9)),
            point(ObjectType::BINARY_OUTPUT, 1, PropertyValue::Enumerated(1)),
            point(ObjectType::ANALOG_VALUE, 2, PropertyValue::Real(1.0)),
        ];
        let changes = compare(&reference, &now);
        let kinds: Vec<(u32, u32, &Difference, bool)> = changes
            .iter()
            .map(|c| (c.object_type, c.instance, &c.difference, c.drift))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (0, 1, &Difference::Value { was: "21.4".into() }, true),
                (
                    4,
                    1,
                    &Difference::Value {
                        was: "inactive".into()
                    },
                    false
                ),
                (2, 2, &Difference::Added, false),
                (3, 9, &Difference::Removed, false),
            ]
        );
    }

    #[test]
    fn a_reference_is_saved_and_loaded_again() {
        let dir = std::env::temp_dir().join(format!("bacnetio-reference-{}", std::process::id()));
        let reference = from_points(
            7,
            UNIX_EPOCH,
            &[point(
                ObjectType::ANALOG_INPUT,
                1,
                PropertyValue::Real(21.4),
            )],
        );
        assert_eq!(load(&dir, 7).unwrap(), None);
        save(&dir, &reference).unwrap();
        let loaded = load(&dir, 7).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(loaded, Some(reference));
    }
}
