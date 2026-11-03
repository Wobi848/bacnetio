//! The data points of a device as EDE file (Engineering Data Exchange, B.I.G.-EU, layout 2.2): the
//! trade's format to hand a point list to a management system or another tool.

use std::time::SystemTime;

use crate::export::{date_time_text, row};
use crate::object::commandable_kind;
use crate::points::Point;

/// The columns of the EDE layout 2.2, in order.
const COLUMNS: [&str; 16] = [
    "keyname",
    "device obj.-instance",
    "object-name",
    "object-type",
    "object-instance",
    "description",
    "present-value-default",
    "min-present-value",
    "max-present-value",
    "settable",
    "supports COV",
    "hi-limit",
    "low-limit",
    "state-text-reference",
    "unit-code",
    "vendor-specific-address",
];

/// State text lists of these devices, each once, in the order they first appear. Objects refer to
/// them by position, from 1.
fn state_tables<'a>(devices: &[(u32, &'a [Point])]) -> Vec<&'a [String]> {
    let mut tables: Vec<&[String]> = Vec::new();
    for point in devices.iter().flat_map(|(_, points)| points.iter()) {
        if !point.state_texts.is_empty() && !tables.contains(&point.state_texts.as_slice()) {
            tables.push(&point.state_texts);
        }
    }
    tables
}

/// The state text file that belongs to [`ede_csv`] of the same devices.
pub fn state_texts_csv(devices: &[(u32, &[Point])]) -> String {
    let tables = state_tables(devices);
    let longest = tables.iter().map(|texts| texts.len()).max().unwrap_or(2);
    let mut head = vec![
        "#Reference Number".to_string(),
        "Text 1 or Inactive-Text".to_string(),
        "Text 2 or Active-Text".to_string(),
    ];
    head.extend((3..=longest).map(|number| format!("Text {number}")));
    let mut out = row(&head);
    for (index, texts) in tables.iter().enumerate() {
        let mut fields = vec![(index + 1).to_string()];
        fields.extend(texts.iter().cloned());
        out.push_str(&row(&fields));
    }
    out
}

/// The EDE file of these devices, written now, with one row per object of their last reading.
pub fn ede_csv(project: &str, devices: &[(u32, &[Point])], now: SystemTime) -> String {
    let today = &date_time_text(now, 0)[..10];
    let mut out = String::new();
    out.push_str("#Engineering-Data-Exchange - B.I.G.-EU\r\n");
    out.push_str(&row(&["PROJECT_NAME", project]));
    out.push_str(&row(&["VERSION_OF_REFERENCEFILE", "1"]));
    out.push_str(&row(&["TIMESTAMP_OF_LAST_CHANGE", today]));
    out.push_str(&row(&["AUTHOR_OF_LAST_CHANGE", "bacnetio"]));
    out.push_str(&row(&["VERSION_OF_LAYOUT", "2.2"]));
    let required = |index: usize| if index < 5 { "mandatory" } else { "optional" };
    let marks: Vec<&str> = (0..COLUMNS.len()).map(required).collect();
    out.push('#');
    out.push_str(&row(&marks));
    out.push_str("# ");
    out.push_str(&row(&COLUMNS));
    let tables = state_tables(devices);
    let rows = devices
        .iter()
        .flat_map(|(device, points)| points.iter().map(move |point| (*device, point)));
    for (device, point) in rows {
        let name = point.name.clone().unwrap_or_default();
        let mut fields = vec![String::new(); COLUMNS.len()];
        fields[0] = format!("{device}_{name}");
        fields[1] = device.to_string();
        fields[2] = name;
        fields[3] = point.object.object_type().to_raw().to_string();
        fields[4] = point.object.instance_number().to_string();
        fields[5] = point.description.clone().unwrap_or_default();
        // what the type allows; whether a value object really takes writes, only the device knows
        fields[9] = if commandable_kind(point.object.object_type()) {
            "Y"
        } else {
            "N"
        }
        .to_string();
        if let Some(index) = tables.iter().position(|texts| {
            !point.state_texts.is_empty() && *texts == point.state_texts.as_slice()
        }) {
            fields[13] = (index + 1).to_string();
        }
        fields[14] = point
            .units
            .map(|units| units.to_raw().to_string())
            .unwrap_or_default();
        out.push_str(&row(&fields));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::enums::{EngineeringUnits, ObjectType};
    use bacnet_types::primitives::ObjectIdentifier;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn header_and_one_row_per_object() {
        let mut point = Point::new(ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 3).unwrap());
        point.name = Some("Supply air temp".into());
        point.description = Some("after the coil".into());
        point.units = Some(EngineeringUnits::DEGREES_CELSIUS);
        let file = ede_csv(
            "Plant A",
            &[(1001, std::slice::from_ref(&point))],
            UNIX_EPOCH + Duration::from_secs(1_793_534_400),
        );
        let lines: Vec<&str> = file.lines().collect();
        assert_eq!(lines[0], "#Engineering-Data-Exchange - B.I.G.-EU");
        assert!(lines[3].contains("2026-11-01"), "{}", lines[3]);
        assert!(lines[5].contains("2.2"));
        assert!(lines[7].starts_with("# keyname;device obj.-instance;object-name"));
        let fields: Vec<&str> = lines[8].split(';').collect();
        assert_eq!(fields.len(), 16);
        assert_eq!(
            &fields[..6],
            [
                "1001_Supply air temp",
                "1001",
                "Supply air temp",
                "0",
                "3",
                "after the coil"
            ]
        );
        assert_eq!(fields[9], "N");
        assert_eq!(fields[14], "62");
        assert!(lines[1].contains("Plant A"));
    }

    #[test]
    fn multi_state_objects_refer_to_their_state_texts() {
        let texts = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
        let point = |instance, list: &[&str]| {
            let mut point =
                Point::new(ObjectIdentifier::new(ObjectType::MULTI_STATE_VALUE, instance).unwrap());
            point.state_texts = texts(list);
            point
        };
        let points = [
            point(1, &["Off", "Auto"]),
            point(2, &["Low", "Mid", "High"]),
            point(3, &["Off", "Auto"]),
            Point::new(ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 1).unwrap()),
        ];
        let devices = [(7, points.as_slice())];
        let file = ede_csv("", &devices, UNIX_EPOCH);
        let references: Vec<&str> = file
            .lines()
            .skip(8)
            .map(|line| line.split(';').nth(13).unwrap())
            .collect();
        assert_eq!(references, ["1", "2", "1", ""]);
        let first: Vec<&str> = file.lines().nth(8).unwrap().split(';').collect();
        assert_eq!(first[9], "Y");
        let states = state_texts_csv(&devices);
        let lines: Vec<&str> = states.lines().collect();
        assert!(lines[0].starts_with("#Reference Number;Text 1"));
        assert!(lines[0].ends_with("Text 3"));
        assert_eq!(lines[1], "1;Off;Auto");
        assert_eq!(lines[2], "2;Low;Mid;High");
    }
}
