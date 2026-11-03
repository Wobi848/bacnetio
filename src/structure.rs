//! The plant view a device carries itself: its Structured View objects (type 29) with their
//! Subordinate_List and Subordinate_Annotations. Read only; a device without them keeps the
//! grouping by object names (concept-explorer section 2).

use bacnet_encoding::constructed::decode_device_object_reference;
use bacnet_types::enums::{ErrorCode, ObjectType, PropertyIdentifier};
use bacnet_types::error::{Error, RemoteError};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::connection::{Client, ReadError, decode_values};

/// One entry of a Structured View's Subordinate_List.
#[derive(Debug, Clone, PartialEq)]
pub struct Subordinate {
    pub object: ObjectIdentifier,
    /// The device that holds the object, where the reference names one; `None` means this device.
    pub device: Option<u32>,
    /// What Subordinate_Annotations says about this entry, where the device has the property.
    pub annotation: Option<String>,
}

/// A Structured View object with what hangs below it.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub object: ObjectIdentifier,
    pub subordinates: Vec<Subordinate>,
    /// The first thing that went wrong reading this view.
    pub error: Option<String>,
}

pub const STRUCTURED_VIEW: ObjectType = ObjectType::STRUCTURED_VIEW;

/// The Structured View objects of `objects`, in the order the object list has them.
pub fn views_of(objects: impl IntoIterator<Item = ObjectIdentifier>) -> Vec<ObjectIdentifier> {
    objects
        .into_iter()
        .filter(|object| object.object_type() == STRUCTURED_VIEW)
        .collect()
}

/// Reads every view's subordinates, one view after the other.
pub async fn read_views(client: &Client, device: u32, views: &[ObjectIdentifier]) -> Vec<View> {
    let mut read = Vec::with_capacity(views.len());
    for object in views {
        read.push(read_view(client, device, *object).await);
    }
    read
}

/// A single view: its subordinates with their annotations. A view whose list cannot be read keeps
/// its place with the reason, so the tree still shows the branch.
pub async fn read_view(client: &Client, device: u32, object: ObjectIdentifier) -> View {
    let list = match read_property(client, device, object, PropertyIdentifier::SUBORDINATE_LIST)
        .await
        .and_then(|bytes| match bytes {
            Some(bytes) => decode_subordinates(&bytes),
            None => Err("the object has no subordinate-list".to_string()),
        }) {
        Ok(list) => list,
        Err(reason) => {
            return View {
                object,
                subordinates: Vec::new(),
                error: Some(reason),
            };
        }
    };
    let annotations = read_property(
        client,
        device,
        object,
        PropertyIdentifier::SUBORDINATE_ANNOTATIONS,
    )
    .await;
    // the annotations are optional: a device without them still gives a usable view
    let texts = match &annotations {
        Ok(Some(bytes)) => decode_annotations(bytes).unwrap_or_default(),
        _ => Vec::new(),
    };
    let subordinates = list
        .into_iter()
        .enumerate()
        .map(|(index, (object, device))| Subordinate {
            object,
            device,
            annotation: texts.get(index).filter(|text| !text.is_empty()).cloned(),
        })
        .collect();
    View {
        object,
        subordinates,
        error: annotations.err(),
    }
}

/// The raw value of a property, or `None` if the object does not have it.
async fn read_property(
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

/// Subordinate_List as the device sent it: one BACnetDeviceObjectReference after the other, each
/// with an optional device.
pub fn decode_subordinates(bytes: &[u8]) -> Result<Vec<(ObjectIdentifier, Option<u32>)>, String> {
    let mut found = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (reference, next) =
            decode_device_object_reference(bytes, offset).map_err(|error| error.to_string())?;
        if next <= offset {
            return Err("an entry of the subordinate-list is empty".to_string());
        }
        found.push((
            reference.object_identifier,
            reference
                .device_identifier
                .map(|device| device.instance_number()),
        ));
        offset = next;
    }
    Ok(found)
}

/// Subordinate_Annotations: one text per entry of the list, in the same order.
fn decode_annotations(bytes: &[u8]) -> Result<Vec<String>, String> {
    Ok(decode_values(bytes)?
        .into_iter()
        .map(|value| match value {
            PropertyValue::CharacterString(text) => text,
            _ => String::new(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_encoding::constructed::encode_device_object_reference;
    use bacnet_encoding::primitives::encode_app_character_string;
    use bacnet_types::constructed::BACnetDeviceObjectReference;
    use bytes::BytesMut;

    fn object(object_type: ObjectType, instance: u32) -> ObjectIdentifier {
        ObjectIdentifier::new(object_type, instance).unwrap()
    }

    fn encoded(references: &[BACnetDeviceObjectReference]) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        for reference in references {
            encode_device_object_reference(&mut bytes, reference);
        }
        bytes.to_vec()
    }

    #[test]
    fn a_subordinate_list_keeps_its_order_and_names_other_devices() {
        let own = BACnetDeviceObjectReference {
            device_identifier: None,
            object_identifier: object(ObjectType::ANALOG_INPUT, 1),
        };
        let remote = BACnetDeviceObjectReference {
            device_identifier: Some(object(ObjectType::DEVICE, 1002)),
            object_identifier: object(ObjectType::STRUCTURED_VIEW, 2),
        };
        let list = decode_subordinates(&encoded(&[own.clone(), remote.clone()])).unwrap();
        assert_eq!(
            list,
            vec![
                (own.object_identifier, None),
                (remote.object_identifier, Some(1002)),
            ]
        );
    }

    #[test]
    fn an_empty_subordinate_list_is_no_error() {
        assert_eq!(decode_subordinates(&[]), Ok(Vec::new()));
    }

    #[test]
    fn nonsense_instead_of_a_reference_is_reported() {
        assert!(decode_subordinates(&[0x21, 0x05]).is_err());
    }

    #[test]
    fn annotations_come_in_the_order_of_the_list() {
        let mut bytes = BytesMut::new();
        encode_app_character_string(&mut bytes, "Air handling").unwrap();
        encode_app_character_string(&mut bytes, "").unwrap();
        assert_eq!(
            decode_annotations(&bytes).unwrap(),
            vec!["Air handling".to_string(), String::new()]
        );
    }

    #[test]
    fn only_structured_views_are_views() {
        let objects = [
            object(ObjectType::ANALOG_INPUT, 1),
            object(ObjectType::STRUCTURED_VIEW, 3),
            object(ObjectType::STRUCTURED_VIEW, 1),
        ];
        assert_eq!(
            views_of(objects),
            vec![
                object(ObjectType::STRUCTURED_VIEW, 3),
                object(ObjectType::STRUCTURED_VIEW, 1),
            ]
        );
    }
}
