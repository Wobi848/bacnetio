//! The lists bacnetio exports, each built once as a [`Table`]: as CSV for scripts and Excel
//! (semicolon, UTF-8 with BOM, CRLF, as in mbusio), and as an Excel file (workbook.rs).

use std::collections::BTreeMap;
use std::time::SystemTime;

use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::alarms;
use crate::event_log::LoggedEvent;
use crate::points::Point;
use crate::present::{
    address_text, date_time_text as device_time_text, entry_action, hex, object_text,
    object_type_text, shown_plain_value, status_text, unit_text, value_with_unit,
};
use crate::record::{Mode, RecordedPoint, Recording};
use crate::scan::Device;
use crate::trend::TrendRecord;
use crate::write::Entry;

const COLUMNS: [&str; 9] = [
    "Device",
    "Object type",
    "Instance",
    "Name",
    "Description",
    "Value",
    "Unit",
    "Status",
    "Error",
];

/// A list to export: its column heads (English, as scripts read them) and its rows of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    fn new<S: AsRef<str>>(columns: &[S]) -> Self {
        Table {
            columns: columns
                .iter()
                .map(|head| head.as_ref().to_string())
                .collect(),
            rows: Vec::new(),
        }
    }

    pub fn csv(&self) -> String {
        // BOM, so Excel reads UTF-8 (°C, m³) correctly
        let mut out = String::from("\u{FEFF}");
        out.push_str(&row(&self.columns));
        for fields in &self.rows {
            out.push_str(&row(fields));
        }
        out
    }
}

pub fn point_list(device: u32, points: &[Point]) -> Table {
    let mut table = Table::new(&COLUMNS);
    table.rows = points
        .iter()
        .map(|point| point_fields(device, point))
        .collect();
    table
}

pub fn point_list_csv(device: u32, points: &[Point]) -> String {
    point_list(device, points).csv()
}

fn point_fields(device: u32, point: &Point) -> Vec<String> {
    vec![
        device.to_string(),
        object_type_text(point.object.object_type()),
        point.object.instance_number().to_string(),
        point.name.clone().unwrap_or_default(),
        point.description.clone().unwrap_or_default(),
        shown_plain_value(point),
        point.units.map(unit_text).unwrap_or_default(),
        status_text(point.status),
        point.error.clone().unwrap_or_default(),
    ]
}

/// What the device answered for an object's priority array.
pub type Priorities = Result<Option<Vec<Option<PropertyValue>>>, String>;

/// The point list with the priority that wins now and every set slot, such as `8=21.5 °C | 16=0`.
pub fn point_priorities(device: u32, points: &[(Point, Priorities)]) -> Table {
    let mut columns = COLUMNS.to_vec();
    columns.extend(["Winning priority", "Set priorities"]);
    let mut table = Table::new(&columns);
    for (point, priorities) in points {
        let mut fields = point_fields(device, point);
        let (winning, set) = priorities_text(point, priorities);
        fields.push(winning);
        fields.push(set);
        table.rows.push(fields);
    }
    table
}

pub fn point_priorities_csv(device: u32, points: &[(Point, Priorities)]) -> String {
    point_priorities(device, points).csv()
}

fn priorities_text(point: &Point, priorities: &Priorities) -> (String, String) {
    let slots = match priorities {
        Ok(Some(slots)) => slots,
        Ok(None) => return (String::new(), String::new()),
        Err(reason) => return (String::new(), format!("not read: {reason}")),
    };
    let held: Vec<(usize, &PropertyValue)> = slots
        .iter()
        .enumerate()
        .filter_map(|(slot, value)| Some((slot + 1, value.as_ref()?)))
        .collect();
    let winning = held.first().map(|(priority, _)| priority.to_string());
    let set: Vec<String> = held
        .iter()
        .map(|(priority, value)| {
            let text = value_with_unit(point.object.object_type(), value, point.units);
            format!("{priority}={text}")
        })
        .collect();
    (winning.unwrap_or_default(), set.join(" | "))
}

const DEVICE_COLUMNS: [&str; 8] = [
    "Device ID",
    "Name",
    "Vendor",
    "Model",
    "Address",
    "Network",
    "MAC",
    "Found by",
];

/// The devices of a scan, for the site documentation.
pub fn device_list(devices: &[Device]) -> Table {
    let mut table = Table::new(&DEVICE_COLUMNS);
    for device in devices {
        table.rows.push(vec![
            device.instance.to_string(),
            device.details.name.clone().unwrap_or_default(),
            device.details.vendor.clone().unwrap_or_default(),
            device.details.model.clone().unwrap_or_default(),
            address_text(device.address),
            device
                .remote
                .as_ref()
                .map(|remote| remote.network.to_string())
                .unwrap_or_default(),
            device
                .remote
                .as_ref()
                .map(|remote| hex(&remote.mac))
                .unwrap_or_default(),
            if device.answered_who_is {
                "Who-Is"
            } else {
                "direct"
            }
            .to_string(),
        ]);
    }
    table
}

pub fn device_list_csv(devices: &[Device]) -> String {
    device_list(devices).csv()
}

/// A trend log read from a device: one row per record, one column per logged point: `titles` names them.
pub fn trend_csv(titles: &[String], records: &[TrendRecord]) -> String {
    let mut out = String::from("\u{FEFF}");
    let mut header = vec!["Time (device)".to_string()];
    header.extend(titles.iter().cloned());
    out.push_str(&row(&header));
    for record in records {
        let mut cells = vec![device_time_text(&record.date, &record.time)];
        cells.extend(
            record
                .values
                .iter()
                .map(|value| value.map(|v| v.to_string()).unwrap_or_default()),
        );
        out.push_str(&row(&cells));
    }
    out
}

/// An event log read from a device, oldest first: `name` names each object as the point list read it.
pub fn event_log_csv(events: &[LoggedEvent], name: impl Fn(ObjectIdentifier) -> String) -> String {
    let mut out = String::from("\u{FEFF}");
    out.push_str(&row(&[
        "Time (device)",
        "Object",
        "From",
        "To",
        "Acknowledged",
        "Message",
    ]));
    for event in events {
        out.push_str(&row(&[
            device_time_text(&event.date, &event.time),
            name(event.object),
            alarms::state_text(event.from),
            alarms::state_text(event.to),
            if event.acknowledged { "yes" } else { "" }.to_string(),
            event.text.clone().unwrap_or_default(),
        ]));
    }
    out
}

/// One row per sampling round, one column per point. `offset_minutes` is the viewer's time zone,
/// so the times read as local times in Excel.
pub fn recording_table(recording: &Recording, offset_minutes: i32) -> Table {
    let series = recording.series();
    let mut header = vec!["Time".to_string()];
    header.extend(series.iter().map(|series| match series.mode {
        Mode::Cov => format!("{} (COV)", column_title(&series.point)),
        Mode::Poll => column_title(&series.point),
    }));
    let columns = series.len();
    let mut rounds: BTreeMap<SystemTime, Vec<Option<f64>>> = BTreeMap::new();
    for (column, series) in series.iter().enumerate() {
        for sample in &series.samples {
            rounds
                .entry(sample.time)
                .or_insert_with(|| vec![None; columns])[column] = sample.value;
        }
    }
    let mut table = Table::new(&header);
    for (time, values) in rounds {
        let mut fields = vec![date_time_text(time, offset_minutes)];
        fields.extend(
            values
                .iter()
                .map(|value| value.map(|v| v.to_string()).unwrap_or_default()),
        );
        table.rows.push(fields);
    }
    table
}

pub fn recording_csv(recording: &Recording, offset_minutes: i32) -> String {
    recording_table(recording, offset_minutes).csv()
}

/// "1001 Analog input 1 Supply air temp (°C)"
fn column_title(point: &RecordedPoint) -> String {
    let mut title = format!("{} {}", point.device, object_text(point.object));
    if let Some(name) = &point.name {
        title.push_str(&format!(" {name}"));
    }
    match point.units.map(unit_text) {
        Some(unit) if !unit.is_empty() => format!("{title} ({unit})"),
        _ => title,
    }
}

const LOG_COLUMNS: [&str; 8] = [
    "Time",
    "Device",
    "Object type",
    "Instance",
    "Priority",
    "Action",
    "Value",
    "Refused",
];

/// Every write of the session, oldest first, as the page lists it; `offset_minutes` as above.
pub fn log_table(entries: &[Entry], offset_minutes: i32) -> Table {
    let mut table = Table::new(&LOG_COLUMNS);
    for entry in entries {
        let (action, value) = entry_action(entry);
        table.rows.push(vec![
            date_time_text(entry.time, offset_minutes),
            entry.device.to_string(),
            object_type_text(entry.object.object_type()),
            entry.object.instance_number().to_string(),
            entry
                .priority
                .map(|priority| priority.to_string())
                .unwrap_or_default(),
            action.to_string(),
            value.unwrap_or_default(),
            entry.error.clone().unwrap_or_default(),
        ]);
    }
    table
}

pub fn log_csv(entries: &[Entry], offset_minutes: i32) -> String {
    log_table(entries, offset_minutes).csv()
}

/// "2026-11-01 09:30:00" at the given offset from UTC.
pub(crate) fn date_time_text(time: SystemTime, offset_minutes: i32) -> String {
    let seconds = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
        + i64::from(offset_minutes) * 60;
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_date(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// Days since 1970-01-01 to year, month, day (Howard Hinnant's `civil_from_days`).
fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

pub(crate) fn row<S: AsRef<str>>(fields: &[S]) -> String {
    let mut line = fields
        .iter()
        .map(|field| quoted(field.as_ref()))
        .collect::<Vec<_>>()
        .join(";");
    line.push_str("\r\n");
    line
}

fn quoted(field: &str) -> String {
    if field.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_lists_each_write_with_its_refusal() {
        use crate::write::Action;
        let object = bacnet_types::primitives::ObjectIdentifier::new(
            bacnet_types::enums::ObjectType::ANALOG_OUTPUT,
            1,
        )
        .unwrap();
        let entry = |action, error: Option<&str>| Entry {
            time: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(3600),
            device: 1001,
            address: None,
            object,
            priority: Some(8),
            action,
            error: error.map(String::from),
        };
        let log = log_csv(
            &[
                entry(Action::Write(PropertyValue::Real(50.0)), None),
                entry(Action::Release, Some("write access denied")),
            ],
            60,
        );
        assert!(
            log.starts_with(
                "\u{FEFF}Time;Device;Object type;Instance;Priority;Action;Value;Refused\r\n"
            ),
            "{log}"
        );
        assert!(
            log.contains("1970-01-01 02:00:00;1001;Analog output;1;8;write;50;\r\n"),
            "{log}"
        );
        assert!(log.contains(";8;release;;write access denied\r\n"), "{log}");
    }

    #[test]
    fn priorities_name_the_winner_and_every_set_slot() {
        let object = bacnet_types::primitives::ObjectIdentifier::new(
            bacnet_types::enums::ObjectType::ANALOG_OUTPUT,
            1,
        )
        .unwrap();
        let mut point = Point::new(object);
        point.name = Some("Valve".into());
        let mut slots = vec![None; 16];
        slots[7] = Some(PropertyValue::Real(40.0));
        slots[15] = Some(PropertyValue::Real(0.0));
        let held = point_priorities_csv(7, &[(point.clone(), Ok(Some(slots)))]);
        assert!(held.contains("Winning priority;Set priorities"), "{held}");
        assert!(held.contains(";8;8=40 | 16=0"), "{held}");
        let unread = point_priorities_csv(7, &[(point.clone(), Err("timeout".into()))]);
        assert!(unread.contains(";;not read: timeout"), "{unread}");
        let none = point_priorities_csv(7, &[(point, Ok(None))]);
        assert!(none.trim_end().ends_with(";;"), "{none}");
    }
    use bacnet_types::enums::{EngineeringUnits, ObjectType};
    use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

    #[test]
    fn csv_with_bom_header_and_quoted_fields() {
        let mut point = Point::new(ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 3).unwrap());
        point.name = Some("Supply; \"air\"".into());
        point.value = Some(PropertyValue::Real(21.4));
        point.units = Some(EngineeringUnits::DEGREES_CELSIUS);

        let csv = point_list_csv(1001, &[point]);

        let mut lines = csv.split("\r\n");
        assert_eq!(
            lines.next(),
            Some("\u{FEFF}Device;Object type;Instance;Name;Description;Value;Unit;Status;Error")
        );
        assert_eq!(
            lines.next(),
            Some("1001;Analog input;3;\"Supply; \"\"air\"\"\";;21.4;°C;;")
        );
    }

    #[test]
    fn dates_from_unix_time() {
        let at = |seconds| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(seconds);
        assert_eq!(date_time_text(at(0), 0), "1970-01-01 00:00:00");
        assert_eq!(date_time_text(at(951_782_400), 0), "2000-02-29 00:00:00");
        assert_eq!(date_time_text(at(1_793_521_800), 60), "2026-11-01 09:30:00");
    }

    #[test]
    fn recording_rounds_become_rows() {
        let mut recording = Recording::default();
        let point = |instance, name: &str| RecordedPoint {
            device: 7,
            object: ObjectIdentifier::new(ObjectType::ANALOG_INPUT, instance).unwrap(),
            name: Some(name.to_string()),
            units: Some(EngineeringUnits::DEGREES_CELSIUS),
        };
        recording.add(point(1, "Supply")).unwrap();
        recording.add(point(2, "Return")).unwrap();
        let first = SystemTime::UNIX_EPOCH;
        let second = first + std::time::Duration::from_secs(10);
        recording.record(
            first,
            &[(point(1, "Supply"), Some(21.5)), (point(2, "Return"), None)],
        );
        recording.record(second, &[(point(1, "Supply"), Some(22.0))]);
        let csv = recording_csv(&recording, 0);
        let lines: Vec<&str> = csv.trim_start_matches('\u{FEFF}').split("\r\n").collect();
        assert_eq!(
            lines[0],
            "Time;7 Analog input 1 Supply (°C);7 Analog input 2 Return (°C)"
        );
        assert_eq!(lines[1], "1970-01-01 00:00:00;21.5;");
        assert_eq!(lines[2], "1970-01-01 00:00:10;22;");
    }

    #[test]
    fn device_list_names_where_each_device_sits() {
        use crate::scan::{Details, RemoteStation};
        use std::net::{Ipv4Addr, SocketAddrV4};
        let device = Device {
            instance: 20101,
            address: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 5), 47808).into(),
            remote: Some(RemoteStation {
                network: 5,
                mac: vec![0x0B],
            }),
            answered_who_is: false,
            details: Details {
                name: Some("VAV 2.01".into()),
                ..Details::default()
            },
            objects: Default::default(),
        };
        let csv = device_list_csv(&[device]);
        assert!(
            csv.contains("20101;VAV 2.01;;;10.0.0.5;5;0B;direct\r\n"),
            "{csv}"
        );
    }
}
