//! One object in depth: its point properties plus who holds its value, from the priority array.

use bacnet_types::constructed::{PropertyReference, ReadAccessSpecification};
use bacnet_types::enums::{ErrorCode, ObjectType, PropertyIdentifier};
use bacnet_types::error::{Error, RemoteError};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::connection::{Client, ReadError, decode_value, decode_values};
use crate::points::{self, Point};

pub const PRIORITIES: u8 = 16;
/// Read one by one when a device cannot answer ReadPropertyMultiple for ALL properties.
const COMMON_PROPERTIES: [PropertyIdentifier; 16] = [
    PropertyIdentifier::OBJECT_NAME,
    PropertyIdentifier::DESCRIPTION,
    PropertyIdentifier::PRESENT_VALUE,
    PropertyIdentifier::UNITS,
    PropertyIdentifier::STATUS_FLAGS,
    PropertyIdentifier::EVENT_STATE,
    PropertyIdentifier::RELIABILITY,
    PropertyIdentifier::OUT_OF_SERVICE,
    PropertyIdentifier::POLARITY,
    PropertyIdentifier::COV_INCREMENT,
    PropertyIdentifier::MIN_PRES_VALUE,
    PropertyIdentifier::MAX_PRES_VALUE,
    PropertyIdentifier::RELINQUISH_DEFAULT,
    PropertyIdentifier::NUMBER_OF_STATES,
    PropertyIdentifier::DEVICE_TYPE,
    PropertyIdentifier::PROFILE_NAME,
];

/// One property as the device sent it.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyReading {
    pub property: PropertyIdentifier,
    /// The values (one for most properties, several for lists), or why there is none.
    pub values: Result<Vec<PropertyValue>, String>,
    /// The answer as the device encoded it, for the productions the flat list of values cannot
    /// express, such as a Notification Class's recipient list; empty where there is none.
    pub encoded: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectDetail {
    pub point: Point,
    /// Slot 0 is priority 1; `None` in a slot means nobody holds it. `None` as a whole means
    /// the object is not commandable.
    pub priority_array: Option<Vec<Option<PropertyValue>>>,
    /// The value when all slots are free.
    pub relinquish_default: Option<PropertyValue>,
}

impl ObjectDetail {
    pub fn commandable(&self) -> bool {
        self.priority_array.is_some()
    }
}

pub async fn read_object(client: &Client, device: u32, object: ObjectIdentifier) -> ObjectDetail {
    let mut point = Point::new(object);
    points::read_single(client, device, &mut point, &|_| {}).await;
    let priority_array = read_priorities(client, device, object).await;
    let relinquish_default = read_optional(
        client,
        device,
        object,
        PropertyIdentifier::RELINQUISH_DEFAULT,
    )
    .await
    .and_then(|bytes| bytes.map(|bytes| decode_value(&bytes)).transpose());
    let priority_array = noted(&mut point, "priority_array", priority_array);
    let relinquish_default = noted(&mut point, "relinquish_default", relinquish_default);
    ObjectDetail {
        point,
        priority_array,
        relinquish_default,
    }
}

/// Object types that can have a priority array and take commands.
pub fn commandable_kind(object_type: ObjectType) -> bool {
    [
        ObjectType::ANALOG_OUTPUT,
        ObjectType::BINARY_OUTPUT,
        ObjectType::MULTI_STATE_OUTPUT,
        ObjectType::ANALOG_VALUE,
        ObjectType::BINARY_VALUE,
        ObjectType::MULTI_STATE_VALUE,
        ObjectType::CHARACTERSTRING_VALUE,
        ObjectType::INTEGER_VALUE,
        ObjectType::LARGE_ANALOG_VALUE,
        ObjectType::POSITIVE_INTEGER_VALUE,
        ObjectType::LIGHTING_OUTPUT,
        ObjectType::BINARY_LIGHTING_OUTPUT,
    ]
    .contains(&object_type)
}

/// The object's priority array: slot 0 is priority 1. `None` when it has none.
pub async fn read_priorities(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> Result<Option<Vec<Option<PropertyValue>>>, String> {
    read_optional(client, device, object, PropertyIdentifier::PRIORITY_ARRAY)
        .await
        .and_then(|bytes| bytes.map(|bytes| priority_slots(&bytes)).transpose())
}

/// Every property of the object: all at once if the device can, else the common ones one by one.
pub async fn read_properties(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> Vec<PropertyReading> {
    let all = ReadAccessSpecification {
        object_identifier: object,
        list_of_property_references: vec![PropertyReference {
            property_identifier: PropertyIdentifier::ALL,
            property_array_index: None,
        }],
    };
    if let Ok(ack) = client
        .read_property_multiple_from_device(device, vec![all])
        .await
    {
        return ack
            .list_of_read_access_results
            .into_iter()
            .flat_map(|result| result.list_of_results)
            .map(|element| PropertyReading {
                property: element.property_identifier,
                encoded: element
                    .property_value
                    .as_ref()
                    .map(|bytes| bytes.to_vec())
                    .unwrap_or_default(),
                values: match (element.property_value, element.error) {
                    (Some(bytes), _) => decode_values(&bytes),
                    (None, Some((class, code))) => {
                        Err(Error::Remote(RemoteError::Error { class, code }).to_string())
                    }
                    (None, None) => Err("empty answer".to_string()),
                },
            })
            .collect();
    }
    let mut readings = Vec::new();
    for property in COMMON_PROPERTIES {
        match read_optional(client, device, object, property).await {
            Ok(None) => {}
            Ok(Some(bytes)) => readings.push(PropertyReading {
                property,
                values: decode_values(&bytes),
                encoded: bytes,
            }),
            Err(reason) => readings.push(PropertyReading {
                property,
                values: Err(reason),
                encoded: Vec::new(),
            }),
        }
    }
    readings
}

/// The value read, or nothing with the reason kept on the point.
fn noted<T>(point: &mut Point, property: &str, read: Result<Option<T>, String>) -> Option<T> {
    read.unwrap_or_else(|reason| {
        point.error.get_or_insert(format!("{property}: {reason}"));
        None
    })
}

/// The raw value of a property, or `None` if the object does not have it.
async fn read_optional(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
    property: PropertyIdentifier,
) -> Result<Option<Vec<u8>>, String> {
    match client
        .read_property_from_device(device, object, property, None)
        .await
    {
        Ok(ack) => Ok(Some(ack.property_value)),
        Err(Error::Remote(RemoteError::Error {
            code: ErrorCode::UNKNOWN_PROPERTY,
            ..
        })) => Ok(None),
        Err(error) => Err(ReadError::Request(error).to_string()),
    }
}

fn priority_slots(bytes: &[u8]) -> Result<Vec<Option<PropertyValue>>, String> {
    let slots: Vec<Option<PropertyValue>> = decode_values(bytes)?
        .into_iter()
        .map(|value| match value {
            PropertyValue::Null => None,
            held => Some(held),
        })
        .collect();
    if slots.len() != usize::from(PRIORITIES) {
        return Err(format!("{} entries instead of {PRIORITIES}", slots.len()));
    }
    Ok(slots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_slots_are_none() {
        // priority 8 holds 21.5, all others are NULL
        let mut bytes = vec![0x00; 7];
        bytes.extend([0x44, 0x41, 0xAC, 0x00, 0x00]);
        bytes.extend([0x00; 8]);
        let slots = priority_slots(&bytes).unwrap();
        assert_eq!(slots[7], Some(PropertyValue::Real(21.5)));
        assert_eq!(slots.iter().filter(|slot| slot.is_some()).count(), 1);
    }

    #[test]
    fn wrong_slot_count_is_reported() {
        assert!(
            priority_slots(&[0x00; 3])
                .unwrap_err()
                .contains("3 entries")
        );
    }
}
