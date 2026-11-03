//! A device's local date and time in words; BACnet allows wildcards in each part.

use bacnet_types::primitives::{Date, Time};

/// "2026-11-01 09:30:00", a device's local date and time.
pub fn date_time_text(date: &Date, time: &Time) -> String {
    format!("{} {}", date_text(date), time_text(time))
}

/// "2026-11-01"; unspecified parts as "*", as BACnet allows wildcards.
pub fn date_text(date: &Date) -> String {
    let part = |value: u8, width: usize, offset: u32| {
        if value == 0xFF {
            "*".to_string()
        } else {
            format!("{:0width$}", u32::from(value) + offset)
        }
    };
    format!(
        "{}-{}-{}",
        part(date.year, 4, 1900),
        part(date.month, 2, 0),
        part(date.day, 2, 0)
    )
}

/// "09:30:00"
pub fn time_text(time: &Time) -> String {
    let part = |value: u8| {
        if value == 0xFF {
            "*".to_string()
        } else {
            format!("{value:02}")
        }
    };
    format!(
        "{}:{}:{}",
        part(time.hour),
        part(time.minute),
        part(time.second)
    )
}
