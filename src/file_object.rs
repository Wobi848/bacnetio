//! A device's File objects read whole with AtomicReadFile, to keep a copy of its programs and
//! parameter sets. Nothing is ever written back: that changes the device's programming.

use anyhow::{Context, bail};
use bacnet_services::file::{FileAccessMethod, FileReadAckMethod};
use bacnet_types::enums::PropertyIdentifier;
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::connection::{Client, read_value};

/// A file is read up to this size; a bigger one would take minutes and is not a parameter set.
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
/// Records asked for at once from a record file; fewer when the answer does not fit.
const RECORDS_PER_REQUEST: u32 = 16;
/// BACnetFileAccessMethod: record access is 0, stream access 1.
const RECORD_ACCESS: u32 = 0;
/// How many requests at most, so a device that never says "end of file" is not asked for ever.
const MAX_REQUESTS: usize = 100_000;

/// The whole file, as the device keeps it. A record file's records each end with a line break, as
/// they usually are lines of text. `octets` is how much a stream answer may carry (limits.rs).
pub async fn read_file(
    client: &Client,
    device: u32,
    file: ObjectIdentifier,
    octets: u32,
) -> anyhow::Result<Vec<u8>> {
    let method = read_value(
        client,
        device,
        file,
        PropertyIdentifier::FILE_ACCESS_METHOD,
        None,
    )
    .await
    .map_err(|error| anyhow::anyhow!("cannot read how the file is accessed: {error}"))?;
    let records = matches!(method, PropertyValue::Enumerated(RECORD_ACCESS));
    let found = client
        .get_device(device)
        .await
        .with_context(|| format!("device {device} is not known; search for it first"))?;
    if found.source_network.is_some() {
        bail!("reading files of devices behind a router is not supported yet");
    }
    let mut content = Vec::new();
    let mut position: i32 = 0;
    let mut size = if records {
        RECORDS_PER_REQUEST
    } else {
        octets.max(1)
    };
    for _ in 0..MAX_REQUESTS {
        let access = if records {
            FileAccessMethod::Record {
                file_start_record: position,
                requested_record_count: size,
            }
        } else {
            FileAccessMethod::Stream {
                file_start_position: position,
                requested_octet_count: size,
            }
        };
        let ack = match client
            .atomic_read_file_decoded(&found.mac_address, file, access)
            .await
        {
            Ok(ack) => ack,
            // too much for one answer: ask for less
            Err(_) if size > 1 => {
                size /= 2;
                continue;
            }
            Err(error) => return Err(error).context("cannot read the file"),
        };
        let read = take(&mut content, ack.access);
        if content.len() > MAX_BYTES {
            bail!("the file is larger than {} MB", MAX_BYTES / 1024 / 1024);
        }
        if ack.end_of_file {
            return Ok(content);
        }
        if read == 0 {
            bail!("the device answers without data and without the end of the file");
        }
        position = position.saturating_add(read as i32);
    }
    bail!("the device does not come to the end of the file")
}

/// Adds what an answer carried; tells how far the position moves: octets or records.
fn take(content: &mut Vec<u8>, access: FileReadAckMethod) -> usize {
    match access {
        FileReadAckMethod::Stream { file_data, .. } => {
            content.extend_from_slice(&file_data);
            file_data.len()
        }
        FileReadAckMethod::Record {
            file_record_data, ..
        } => {
            for record in &file_record_data {
                content.extend_from_slice(record);
                if !record.ends_with(b"\n") {
                    content.push(b'\n');
                }
            }
            file_record_data.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_become_lines_and_streams_stay_as_they_are() {
        let mut content = Vec::new();
        let moved = take(
            &mut content,
            FileReadAckMethod::Record {
                file_start_record: 0,
                returned_record_count: 2,
                file_record_data: vec![b"first".to_vec(), b"second\n".to_vec()],
            },
        );
        assert_eq!(moved, 2);
        assert_eq!(content, b"first\nsecond\n");
        let moved = take(
            &mut content,
            FileReadAckMethod::Stream {
                file_start_position: 0,
                file_data: vec![0, 1, 2],
            },
        );
        assert_eq!(moved, 3);
        assert_eq!(&content[13..], &[0, 1, 2]);
    }
}
