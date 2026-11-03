//! Schedule and calendar objects in readable form: the week, its exceptions, the calendar's dates.

use anyhow::{Context, anyhow};
use bacnet_encoding::constructed::{
    decode_calendar_entry, decode_daily_schedule, decode_special_event, encode_daily_schedule,
};
use bacnet_types::constructed::{BACnetCalendarEntry, BACnetTimeValue, SpecialEventPeriod};
use bacnet_types::enums::PropertyIdentifier;
use bacnet_types::primitives::{Date, ObjectIdentifier, PropertyValue, Time};

use bytes::BytesMut;

use crate::connection::{Client, read_value};
use crate::present::{date_text, object_text, plain_text, time_text};
use crate::property_text::property_name;

#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub time: String,
    pub value: String,
    /// As typed to change the day: nothing rounded, so an edit leaves the other changes as they are.
    pub exact: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exception {
    pub when: String,
    pub priority: u64,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScheduleView {
    /// Monday to Sunday.
    pub week: Vec<Vec<Change>>,
    pub exceptions: Vec<Exception>,
}

async fn read_raw(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
    property: PropertyIdentifier,
) -> anyhow::Result<Vec<u8>> {
    let answer = client
        .read_property_from_device(device, object, property, None)
        .await
        .with_context(|| format!("cannot read {}", property_name(property)))?;
    Ok(answer.property_value.to_vec())
}

fn change(object: ObjectIdentifier, time: &Time, value: &PropertyValue) -> Change {
    let exact_value = match value {
        PropertyValue::Real(number) => number.to_string(),
        PropertyValue::Double(number) => number.to_string(),
        other => plain_text(object.object_type(), other),
    };
    let exact_time = if time.second == 0 {
        format!("{:02}:{:02}", time.hour, time.minute)
    } else {
        format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second)
    };
    Change {
        time: time_text(time),
        value: plain_text(object.object_type(), value),
        exact: format!("{exact_time} {exact_value}"),
    }
}

fn entry_text(entry: &BACnetCalendarEntry) -> String {
    match entry {
        BACnetCalendarEntry::Date(date) => any_date(date),
        BACnetCalendarEntry::DateRange(range) => {
            format!(
                "{} – {}",
                any_date(&range.start_date),
                any_date(&range.end_date)
            )
        }
        BACnetCalendarEntry::WeekNDay(week) => format!(
            "month {}, week {}, weekday {}",
            wildcard(week.month),
            wildcard(week.week_of_month),
            wildcard(week.day_of_week)
        ),
    }
}

const fn is_any(value: u8) -> bool {
    value == 0xFF
}

fn wildcard(value: u8) -> String {
    if is_any(value) {
        "any".to_string()
    } else {
        value.to_string()
    }
}

/// A date with parts that say "any", as schedules use them.
fn any_date(date: &Date) -> String {
    if is_any(date.year) || is_any(date.month) || is_any(date.day) {
        let year = if is_any(date.year) {
            "any".to_string()
        } else {
            (u32::from(date.year) + 1900).to_string()
        };
        return format!("{year}-{}-{}", wildcard(date.month), wildcard(date.day));
    }
    date_text(date)
}

/// The week from the bytes of Weekly_Schedule: seven daily schedules in a row.
pub fn decode_week(object: ObjectIdentifier, bytes: &[u8]) -> anyhow::Result<Vec<Vec<Change>>> {
    let mut week = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (values, next) =
            decode_daily_schedule(bytes, offset).map_err(|error| anyhow!("{error}"))?;
        week.push(
            values
                .iter()
                .map(|item| change(object, &item.time, &item.value))
                .collect(),
        );
        offset = next;
    }
    Ok(week)
}

fn decode_exceptions(object: ObjectIdentifier, bytes: &[u8]) -> anyhow::Result<Vec<Exception>> {
    let mut exceptions = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (event, next) =
            decode_special_event(bytes, offset).map_err(|error| anyhow!("{error}"))?;
        exceptions.push(Exception {
            when: match &event.period {
                SpecialEventPeriod::CalendarEntry(entry) => entry_text(entry),
                SpecialEventPeriod::CalendarReference(calendar) => object_text(*calendar),
            },
            priority: event.event_priority,
            changes: event
                .list_of_time_values
                .iter()
                .map(|item| change(object, &item.time, &item.value))
                .collect(),
        });
        offset = next;
    }
    Ok(exceptions)
}

/// A schedule's week and exceptions. A schedule may have only one of them; the other stays empty.
pub async fn read_schedule(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> anyhow::Result<ScheduleView> {
    let week = read_raw(client, device, object, PropertyIdentifier::WEEKLY_SCHEDULE).await;
    let exceptions = read_raw(
        client,
        device,
        object,
        PropertyIdentifier::EXCEPTION_SCHEDULE,
    )
    .await;
    if let (Err(first), Err(_)) = (&week, &exceptions) {
        return Err(anyhow!("{first:#}"));
    }
    Ok(ScheduleView {
        week: week.map_or(Ok(Vec::new()), |bytes| decode_week(object, &bytes))?,
        exceptions: exceptions.map_or(Ok(Vec::new()), |bytes| decode_exceptions(object, &bytes))?,
    })
}

/// A calendar's dates, each in words.
pub async fn read_calendar(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> anyhow::Result<Vec<String>> {
    let bytes = read_raw(client, device, object, PropertyIdentifier::DATE_LIST).await?;
    let mut dates = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (entry, next) =
            decode_calendar_entry(&bytes, offset).map_err(|error| anyhow!("{error}"))?;
        dates.push(entry_text(&entry));
        offset = next;
    }
    Ok(dates)
}

/// A value for a schedule, of the same kind as its default value.
fn parse_value(kind: &PropertyValue, text: &str) -> Result<PropertyValue, String> {
    let invalid = || format!("\"{text}\" is no value for this schedule");
    match kind {
        PropertyValue::Real(_) => text
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|value: &f32| value.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(invalid),
        PropertyValue::Double(_) => text
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|value: &f64| value.is_finite())
            .map(PropertyValue::Double)
            .ok_or_else(invalid),
        PropertyValue::Unsigned(_) => text
            .trim_start_matches("state ")
            .parse()
            .map(PropertyValue::Unsigned)
            .map_err(|_| invalid()),
        PropertyValue::Signed(_) => text
            .parse()
            .map(PropertyValue::Signed)
            .map_err(|_| invalid()),
        PropertyValue::Boolean(_) => match text {
            "true" => Ok(PropertyValue::Boolean(true)),
            "false" => Ok(PropertyValue::Boolean(false)),
            _ => Err(invalid()),
        },
        PropertyValue::Enumerated(_) => match text {
            "inactive" => Ok(PropertyValue::Enumerated(0)),
            "active" => Ok(PropertyValue::Enumerated(1)),
            _ => text
                .parse()
                .map(PropertyValue::Enumerated)
                .map_err(|_| invalid()),
        },
        _ => Err("this schedule's kind of value cannot be edited here".to_string()),
    }
}

/// A day from what was typed: "06:00 21.5, 18:00 17". Empty means nothing happens on that day.
pub fn parse_day(kind: &PropertyValue, text: &str) -> Result<Vec<BACnetTimeValue>, String> {
    let mut day = Vec::new();
    for part in text
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let (time, value) = part
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("\"{part}\" needs a time and a value"))?;
        let numbers: Vec<Option<u8>> = time.split(':').map(|number| number.parse().ok()).collect();
        let time = match numbers.as_slice() {
            [Some(hour), Some(minute)] if *hour < 24 && *minute < 60 => Time {
                hour: *hour,
                minute: *minute,
                second: 0,
                hundredths: 0,
            },
            [Some(hour), Some(minute), Some(second)]
                if *hour < 24 && *minute < 60 && *second < 60 =>
            {
                Time {
                    hour: *hour,
                    minute: *minute,
                    second: *second,
                    hundredths: 0,
                }
            }
            _ => return Err(format!("\"{time}\" is no time (such as 06:30)")),
        };
        day.push(BACnetTimeValue {
            time,
            value: parse_value(kind, value.trim())?,
        });
    }
    day.sort_by_key(|item| (item.time.hour, item.time.minute, item.time.second));
    Ok(day)
}

/// Writes one day of the week (0 is Monday), leaving the other days as they are.
pub async fn write_day(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
    day: u32,
    text: &str,
) -> anyhow::Result<()> {
    let kind = read_value(
        client,
        device,
        object,
        PropertyIdentifier::SCHEDULE_DEFAULT,
        None,
    )
    .await
    .map_err(|error| anyhow!("cannot read the schedule's default value: {error}"))?;
    let values = parse_day(&kind, text).map_err(|reason| anyhow!(reason))?;
    let mut encoded = BytesMut::new();
    encode_daily_schedule(&mut encoded, &values).map_err(|error| anyhow!("{error}"))?;
    client
        .write_property_to_device(
            device,
            object,
            PropertyIdentifier::WEEKLY_SCHEDULE,
            Some(day + 1),
            encoded.to_vec(),
            None,
        )
        .await
        .map_err(|error| anyhow!("the device refused: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_a_day_leaves_its_other_changes_exact() {
        let time = Time {
            hour: 6,
            minute: 30,
            second: 30,
            hundredths: 0,
        };
        let object = ObjectIdentifier::new(bacnet_types::enums::ObjectType::SCHEDULE, 1).unwrap();
        let change = change(object, &time, &PropertyValue::Real(21.456));
        assert_eq!(change.exact, "06:30:30 21.456");
        let day = parse_day(&PropertyValue::Real(0.0), &change.exact).unwrap();
        assert_eq!(day[0].time.second, 30);
        assert_eq!(day[0].value, PropertyValue::Real(21.456));
    }

    #[test]
    fn a_typed_day_becomes_sorted_changes_of_the_schedule_s_kind() {
        let day = parse_day(&PropertyValue::Real(0.0), "18:00 17, 06:30 21,5; ").unwrap_err();
        assert!(
            day.contains("needs a time") || day.contains("no value"),
            "{day}"
        );
        let day = parse_day(&PropertyValue::Real(0.0), "18:00 17; 06:30 21,5").unwrap();
        assert_eq!(day[0].time.hour, 6);
        assert_eq!(day[0].value, PropertyValue::Real(21.5));
        assert_eq!(day[1].value, PropertyValue::Real(17.0));
        assert_eq!(
            parse_day(&PropertyValue::Real(0.0), " ").unwrap(),
            Vec::new()
        );
        assert!(parse_day(&PropertyValue::Real(0.0), "25:00 1").is_err());
        assert!(parse_day(&PropertyValue::Real(0.0), "06:00 NaN").is_err());
        let binary = parse_day(&PropertyValue::Enumerated(0), "06:00 active").unwrap();
        assert_eq!(binary[0].value, PropertyValue::Enumerated(1));
        let states = parse_day(&PropertyValue::Unsigned(1), "06:00 state 2").unwrap();
        assert_eq!(states[0].value, PropertyValue::Unsigned(2));
    }
}
