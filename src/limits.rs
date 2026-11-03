//! Request sizes planned from what a device says it can take: its largest answer (max APDU) and
//! whether it sends long answers in segments. A request too large for a device without segmentation
//! is refused at best, and on some devices runs into a timeout, which costs seconds each over a
//! VPN (concept-explorer section 2.2).

use bacnet_types::constructed::{PropertyReference, ReadAccessSpecification};
use bacnet_types::enums::{ObjectType, PropertyIdentifier, Segmentation};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::connection::{Client, decode_value};

/// Header of a ReadPropertyMultiple answer and the object around the results, roughly.
const ANSWER_OVERHEAD: usize = 24;
/// One object of a point reading in an answer: name, description, value, units, status flags.
const BYTES_PER_POINT: usize = 140;
/// One object list entry in a ReadPropertyMultiple answer: property, index and object identifier.
const BYTES_PER_LIST_ENTRY: usize = 12;
/// One object identifier in the object list read whole.
const BYTES_PER_IDENTIFIER: usize = 5;
/// One trend log record with date, time, value and status.
const BYTES_PER_RECORD: usize = 24;
/// Upper bounds, also for devices that segment: small requests keep the first values coming fast.
const MOST_POINTS: usize = 16;
const MOST_LIST_ENTRIES: usize = 100;
const MOST_RECORDS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// Largest answer the device sends in one piece, in bytes.
    pub max_apdu: usize,
    /// The device sends longer answers in segments.
    pub segments: bool,
}

impl Limits {
    /// What bacnetio assumes when the device does not say: a full Ethernet APDU, with segmentation.
    /// Requests that are still too large are made smaller as they fail.
    pub const ASSUMED: Limits = Limits {
        max_apdu: 1476,
        segments: true,
    };

    fn fitting(&self, bytes_per_item: usize, most: usize) -> usize {
        if self.segments {
            return most;
        }
        (self.max_apdu.saturating_sub(ANSWER_OVERHEAD) / bytes_per_item).clamp(1, most)
    }

    /// Objects per ReadPropertyMultiple when reading points.
    pub fn points_per_request(&self) -> usize {
        self.fitting(BYTES_PER_POINT, MOST_POINTS)
    }

    /// Object list entries per ReadPropertyMultiple when the list is read in parts.
    pub fn list_entries_per_request(&self) -> u32 {
        self.fitting(BYTES_PER_LIST_ENTRY, MOST_LIST_ENTRIES) as u32
    }

    /// Whether an object list of `count` entries comes in one answer.
    pub fn whole_list_fits(&self, count: u32) -> bool {
        self.segments || ANSWER_OVERHEAD + count as usize * BYTES_PER_IDENTIFIER <= self.max_apdu
    }

    /// Trend log records per ReadRange.
    pub fn records_per_request(&self) -> u16 {
        self.fitting(BYTES_PER_RECORD, MOST_RECORDS) as u16
    }
}

/// The device's limits from its device object, in one request; `None` when it does not tell.
pub async fn read_limits(client: &Client, instance: u32) -> Option<Limits> {
    let device = ObjectIdentifier::new(ObjectType::DEVICE, instance).ok()?;
    let spec = ReadAccessSpecification {
        object_identifier: device,
        list_of_property_references: [
            PropertyIdentifier::MAX_APDU_LENGTH_ACCEPTED,
            PropertyIdentifier::SEGMENTATION_SUPPORTED,
        ]
        .into_iter()
        .map(|property_identifier| PropertyReference {
            property_identifier,
            property_array_index: None,
        })
        .collect(),
    };
    let ack = client
        .read_property_multiple_from_device(instance, vec![spec])
        .await
        .ok()?;
    let mut max_apdu = None;
    let mut segmentation = None;
    for element in ack
        .list_of_read_access_results
        .into_iter()
        .flat_map(|result| result.list_of_results)
    {
        let value = element
            .property_value
            .and_then(|bytes| decode_value(&bytes).ok());
        match (element.property_identifier, value) {
            (
                PropertyIdentifier::MAX_APDU_LENGTH_ACCEPTED,
                Some(PropertyValue::Unsigned(bytes)),
            ) => {
                max_apdu = usize::try_from(bytes).ok();
            }
            (PropertyIdentifier::SEGMENTATION_SUPPORTED, Some(PropertyValue::Enumerated(raw))) => {
                segmentation = u8::try_from(raw).ok().map(Segmentation::from_raw);
            }
            _ => {}
        }
    }
    Some(Limits {
        max_apdu: max_apdu?,
        segments: matches!(segmentation?, Segmentation::BOTH | Segmentation::TRANSMIT),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_without_segmentation_gets_requests_that_fit() {
        let small = Limits {
            max_apdu: 480,
            segments: false,
        };
        assert_eq!(small.points_per_request(), 3);
        assert_eq!(small.list_entries_per_request(), 38);
        assert_eq!(small.records_per_request(), 19);
        assert!(small.whole_list_fits(80));
        assert!(!small.whole_list_fits(170));
        let large = Limits {
            max_apdu: 1476,
            segments: false,
        };
        assert_eq!(large.points_per_request(), 10);
        assert!(!large.whole_list_fits(455));
    }

    #[test]
    fn a_segmenting_device_gets_the_upper_bounds() {
        assert_eq!(Limits::ASSUMED.points_per_request(), MOST_POINTS);
        assert_eq!(
            Limits::ASSUMED.list_entries_per_request(),
            MOST_LIST_ENTRIES as u32
        );
        assert!(Limits::ASSUMED.whole_list_fits(10_000));
        assert_eq!(Limits::ASSUMED.records_per_request(), MOST_RECORDS as u16);
    }
}
