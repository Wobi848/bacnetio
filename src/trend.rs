//! The device's own recordings: Trend Log objects, read page by page with ReadRange.

use anyhow::{Context, bail};
use bacnet_client::log_reader::LogCursor;
use bacnet_services::read_range::LogRecords;
use bacnet_types::constructed::{BACnetLogRecord, LogDatum};
use bacnet_types::primitives::{Date, ObjectIdentifier, Time};

use crate::connection::Client;

/// A log is read up to this many records; big buffers would take minutes over MS/TP.
pub const MAX_RECORDS: usize = 20_000;
const MAX_PAGES: usize = 20_000;

/// One logged value with the device's local date and time.
#[derive(Debug, Clone, PartialEq)]
pub struct TrendRecord {
    pub date: Date,
    pub time: Time,
    /// `None` when the device logged a failure or a value that is not a number.
    pub value: Option<f64>,
}

#[derive(Debug, Default)]
pub struct TrendLog {
    pub records: Vec<TrendRecord>,
    /// Records the device dropped while it was read, or before.
    pub gaps: bool,
    /// Stopped at [`MAX_RECORDS`].
    pub truncated: bool,
}

/// All records of a Trend Log object, oldest first, `page` records per ReadRange at most
/// (limits.rs). A page the device cannot answer is halved at once.
pub async fn read_trend_log(
    client: &Client,
    device: u32,
    log: ObjectIdentifier,
    page: u16,
) -> anyhow::Result<TrendLog> {
    let found = client
        .get_device(device)
        .await
        .with_context(|| format!("device {device} is not known; search for it first"))?;
    if found.source_network.is_some() {
        bail!("reading trend logs of devices behind a router is not supported yet");
    }
    let mut result = TrendLog::default();
    let mut cursor = LogCursor::Oldest;
    let mut size = page.max(1);
    for _ in 0..MAX_PAGES {
        let page = match client
            .read_log_page(&found.mac_address, log, cursor, size)
            .await
        {
            Ok(page) => page,
            Err(_) if size > 1 => {
                size /= 2;
                continue;
            }
            Err(error) => return Err(error).context("cannot read the log buffer"),
        };
        result.gaps |= page.gap.is_some();
        let LogRecords::TrendLog(records) = page.records else {
            bail!("this object does not hold trend log records");
        };
        result
            .records
            .extend(records.iter().filter_map(trend_record));
        if result.records.len() >= MAX_RECORDS {
            result.records.truncate(MAX_RECORDS);
            result.truncated = !page.done;
            return Ok(result);
        }
        if page.done {
            return Ok(result);
        }
        cursor = page.next;
    }
    // a device whose cursor never reaches the end would otherwise be asked for ever
    result.truncated = true;
    Ok(result)
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
        value,
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
        let value = |datum| trend_record(&record(datum)).map(|found| found.value);
        assert_eq!(value(LogDatum::RealValue(21.4)), Some(Some(21.4)));
        assert_eq!(value(LogDatum::BooleanValue(true)), Some(Some(1.0)));
        assert_eq!(value(LogDatum::NullValue), Some(None));
        assert_eq!(value(LogDatum::TimeChange(3.0)), None);
    }
}
