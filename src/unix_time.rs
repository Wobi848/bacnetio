//! A moment as the files and the API keep it: seconds or milliseconds since 1970, and 0 for a
//! clock set before then.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

pub fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}
