//! A log object's buffer read page by page with ReadRange: Trend Log, Trend Log Multiple and Event
//! Log keep their records this way.

use anyhow::{Context, bail};
use bacnet_client::log_reader::LogCursor;
use bacnet_services::read_range::LogRecords;
use bacnet_types::primitives::ObjectIdentifier;

use crate::connection::Client;

/// A log is read up to this many records; big buffers would take minutes over MS/TP.
pub const MAX_RECORDS: usize = 20_000;
const MAX_PAGES: usize = 20_000;

#[derive(Debug)]
pub struct Buffer<T> {
    /// Oldest first.
    pub records: Vec<T>,
    /// Records the device dropped while it was read, or before.
    pub gaps: bool,
    /// Stopped at [`MAX_RECORDS`].
    pub truncated: bool,
}

impl<T> Default for Buffer<T> {
    fn default() -> Self {
        Buffer {
            records: Vec::new(),
            gaps: false,
            truncated: false,
        }
    }
}

/// All records of the log, oldest first, `page` records per ReadRange at most (limits.rs). A page
/// the device cannot answer is halved at once. `take` turns a page into records, or `None` when the
/// page holds records of another kind of log; `kind` then names what was expected.
pub async fn read_buffer<T>(
    client: &Client,
    device: u32,
    log: ObjectIdentifier,
    page: u16,
    kind: &str,
    take: impl Fn(LogRecords) -> Option<Vec<T>>,
) -> anyhow::Result<Buffer<T>> {
    let found = client
        .get_device(device)
        .await
        .with_context(|| format!("device {device} is not known; search for it first"))?;
    if found.source_network.is_some() {
        bail!("reading {kind}s of devices behind a router is not supported yet");
    }
    let mut result = Buffer::default();
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
        let Some(records) = take(page.records) else {
            bail!("this object does not hold {kind} records");
        };
        result.records.extend(records);
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
