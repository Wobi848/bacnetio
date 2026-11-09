//! The device's own recordings: Trend Log and Trend Log Multiple objects, read page by page with
//! ReadRange.

use anyhow::Context;
use bacnet_encoding::constructed::decode_device_object_property_reference;
use bacnet_services::read_range::LogRecords;
use bacnet_types::constructed::{
    BACnetDeviceObjectPropertyReference, BACnetLogMultipleRecord, BACnetLogRecord, LogData,
    LogDatum, LogValue,
};
use bacnet_types::enums::PropertyIdentifier;
use bacnet_types::primitives::{Date, ObjectIdentifier, Time};

use crate::connection::Client;
use crate::log_buffer::{Buffer, read_buffer};

/// One logged round with the device's local date and time: one value for a Trend Log, one per
/// logged point for a Trend Log Multiple.
#[derive(Debug, Clone, PartialEq)]
pub struct TrendRecord {
    pub date: Date,
    pub time: Time,
    /// `None` where the device logged a failure or a value that is not a number.
    pub values: Vec<Option<f64>>,
}

pub type TrendLog = Buffer<TrendRecord>;

/// All records of a Trend Log or Trend Log Multiple object, oldest first.
pub async fn read_trend_log(
    client: &Client,
    device: u32,
    log: ObjectIdentifier,
    page: u16,
) -> anyhow::Result<TrendLog> {
    read_buffer(
        client,
        device,
        log,
        page,
        "trend log",
        |records| match records {
            LogRecords::TrendLog(records) => {
                Some(records.iter().filter_map(trend_record).collect())
            }
            LogRecords::TrendLogMultiple(records) => {
                Some(records.iter().filter_map(multiple_record).collect())
            }
            _ => None,
        },
    )
    .await
}

/// What a Trend Log Multiple logs, in the order of its records' values: Log_DeviceObjectProperty.
pub async fn read_members(
    client: &Client,
    device: u32,
    log: ObjectIdentifier,
) -> anyhow::Result<Vec<BACnetDeviceObjectPropertyReference>> {
    let ack = client
        .read_property_from_device(
            device,
            log,
            PropertyIdentifier::LOG_DEVICE_OBJECT_PROPERTY,
            None,
        )
        .await
        .context("cannot read which points the log records")?;
    decode_members(&ack.property_value)
}

fn decode_members(bytes: &[u8]) -> anyhow::Result<Vec<BACnetDeviceObjectPropertyReference>> {
    let mut members = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (member, next) = decode_device_object_property_reference(bytes, offset)
            .context("cannot read which points the log records")?;
        members.push(member);
        offset = next;
    }
    Ok(members)
}

/// A round of a Trend Log Multiple; status changes of the log itself and clock changes are left
/// out.
fn multiple_record(record: &BACnetLogMultipleRecord) -> Option<TrendRecord> {
    let LogData::Values(values) = &record.log_data else {
        return None;
    };
    Some(TrendRecord {
        date: record.date,
        time: record.time,
        values: values.iter().map(member_value).collect(),
    })
}

fn member_value(value: &LogValue) -> Option<f64> {
    match value {
        LogValue::RealValue(value) => value.to_string().parse().ok(),
        LogValue::UnsignedValue(value) | LogValue::EnumValue(value) => Some(*value as f64),
        LogValue::SignedValue(value) => Some(*value as f64),
        LogValue::BooleanValue(value) => Some(f64::from(u8::from(*value))),
        LogValue::Failure { .. } | LogValue::NullValue | LogValue::AnyValue(_) => None,
        LogValue::BitstringValue { .. } => None,
    }
}

/// A value record; status changes of the log itself and clock changes are left out.
fn trend_record(record: &BACnetLogRecord) -> Option<TrendRecord> {
    let value = match &record.log_datum {
        LogDatum::RealValue(value) => value.to_string().parse().ok(),
        LogDatum::UnsignedValue(value) | LogDatum::EnumValue(value) => Some(*value as f64),
        LogDatum::SignedValue(value) => Some(*value as f64),
        LogDatum::BooleanValue(value) => Some(f64::from(u8::from(*value))),
        LogDatum::Failure { .. } | LogDatum::NullValue | LogDatum::AnyValue(_) => None,
        LogDatum::BitstringValue { .. } => None,
        LogDatum::LogStatus(_) | LogDatum::TimeChange(_) => return None,
    };
    Some(TrendRecord {
        date: record.date,
        time: record.time,
        values: vec![value],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(log_datum: LogDatum) -> BACnetLogRecord {
        BACnetLogRecord {
            date: Date {
                year: 126,
                month: 11,
                day: 1,
                day_of_week: 7,
            },
            time: Time {
                hour: 9,
                minute: 30,
                second: 0,
                hundredths: 0,
            },
            log_datum,
            status_flags: None,
        }
    }

    #[test]
    fn values_become_numbers_and_log_events_are_left_out() {
        let value = |datum| trend_record(&record(datum)).map(|found| found.values);
        assert_eq!(value(LogDatum::RealValue(21.4)), Some(vec![Some(21.4)]));
        assert_eq!(value(LogDatum::BooleanValue(true)), Some(vec![Some(1.0)]));
        assert_eq!(value(LogDatum::NullValue), Some(vec![None]));
        assert_eq!(value(LogDatum::TimeChange(3.0)), None);
    }

    #[test]
    fn a_round_of_several_points_keeps_their_order_and_their_failures() {
        let round = |log_data| {
            let single = record(LogDatum::NullValue);
            multiple_record(&BACnetLogMultipleRecord {
                date: single.date,
                time: single.time,
                log_data,
            })
            .map(|found| found.values)
        };
        let failure = LogValue::Failure {
            error_class: bacnet_types::enums::ErrorClass::OBJECT,
            error_code: bacnet_types::enums::ErrorCode::UNKNOWN_OBJECT,
        };
        assert_eq!(
            round(LogData::Values(vec![
                LogValue::RealValue(21.5),
                failure,
                LogValue::BooleanValue(true)
            ])),
            Some(vec![Some(21.5), None, Some(1.0)])
        );
        assert_eq!(round(LogData::TimeChange(1.0)), None);
    }

    #[test]
    fn the_points_logged_are_read_from_their_references() {
        let first =
            ObjectIdentifier::new(bacnet_types::enums::ObjectType::ANALOG_INPUT, 1).unwrap();
        let mut bytes = Vec::new();
        for reference in [
            BACnetDeviceObjectPropertyReference::new_local(first, 85),
            BACnetDeviceObjectPropertyReference::new_local(first, 77),
        ] {
            let mut buffer = bytes::BytesMut::new();
            bacnet_encoding::constructed::encode_device_object_property_reference(
                &mut buffer,
                &reference,
            );
            bytes.extend_from_slice(&buffer);
        }
        let members = decode_members(&bytes).unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[1].property_identifier, 77);
    }
}
