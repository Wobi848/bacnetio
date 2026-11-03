//! A device's object list: in one piece where the device can send it, else in parts, as many
//! entries per request as its limits allow (limits.rs). Devices without segmentation cannot send a
//! long list at once.

use anyhow::{Context, bail};
use bacnet_types::constructed::{PropertyReference, ReadAccessSpecification};
use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::error::{Error, RemoteError};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::connection::{Client, decode_value, decode_values, read_value};
use crate::limits::Limits;
use crate::observe::{Event, Observer, timed};
use crate::present::value_kind_text;

/// The object list, and whether it had to be read in parts. A device that segments is asked for
/// the whole list first; one that does not only when the list fits into one answer.
pub async fn read(
    client: &Client,
    instance: u32,
    limits: Limits,
    observe: Observer<'_>,
) -> anyhow::Result<(Vec<ObjectIdentifier>, bool)> {
    let device = ObjectIdentifier::new(ObjectType::DEVICE, instance)
        .with_context(|| format!("{instance} is not a valid device ID"))?;
    let mut whole_failed = None;
    if limits.segments {
        match read_whole(client, instance, device, observe).await {
            Ok(objects) if complete(client, instance, device, &objects, observe).await => {
                return Ok((objects, false));
            }
            Ok(objects) => {
                whole_failed = Some(format!(
                    "the whole list came with only {} entries",
                    objects.len()
                ));
            }
            Err(reason) => whole_failed = Some(reason),
        }
    }
    let count = timed(
        observe,
        || "ReadProperty object-list[0]".into(),
        read_value(
            client,
            instance,
            device,
            PropertyIdentifier::OBJECT_LIST,
            Some(0),
        ),
    )
    .await;
    let count = match count {
        Ok(PropertyValue::Unsigned(count)) => {
            u32::try_from(count).with_context(|| format!("the device reports {count} objects"))?
        }
        Ok(other) => bail!(
            "the device reports the object count as {}, not as a whole number",
            value_kind_text(&other)
        ),
        Err(error) => match whole_failed {
            Some(whole) => bail!("cannot read the object list ({whole}), nor its length ({error})"),
            None => bail!("cannot read the length of the object list: {error}"),
        },
    };
    observe(Event::Listed {
        read: 0,
        total: Some(count as usize),
    });
    if !limits.segments
        && limits.whole_list_fits(count)
        && let Ok(objects) = read_whole(client, instance, device, observe).await
        && objects.len() == count as usize
    {
        return Ok((objects, false));
    }
    let entries = read_in_parts(client, instance, device, count, limits, observe).await?;
    Ok((entries, true))
}

/// Whether a list read in one piece has as many entries as the device says it holds. A device might
/// cut a long answer short without a word, and objects would be missing unnoticed. One that does not
/// tell its count gets the benefit of the doubt.
async fn complete(
    client: &Client,
    instance: u32,
    device: ObjectIdentifier,
    objects: &[ObjectIdentifier],
    observe: Observer<'_>,
) -> bool {
    let count = timed(
        observe,
        || "ReadProperty object-list[0], to check the whole list".into(),
        read_value(
            client,
            instance,
            device,
            PropertyIdentifier::OBJECT_LIST,
            Some(0),
        ),
    )
    .await;
    !matches!(count, Ok(PropertyValue::Unsigned(count)) if count != objects.len() as u64)
}

async fn read_whole(
    client: &Client,
    instance: u32,
    device: ObjectIdentifier,
    observe: Observer<'_>,
) -> Result<Vec<ObjectIdentifier>, String> {
    let answer = timed(
        observe,
        || "ReadProperty object-list".into(),
        client.read_property_from_device(instance, device, PropertyIdentifier::OBJECT_LIST, None),
    )
    .await
    .map_err(|error| error.to_string())?;
    decode(&answer.property_value)
}

/// Entries 1 to `count`, as many per request as fit; one by one where the device knows no
/// ReadPropertyMultiple.
async fn read_in_parts(
    client: &Client,
    instance: u32,
    device: ObjectIdentifier,
    count: u32,
    limits: Limits,
    observe: Observer<'_>,
) -> anyhow::Result<Vec<ObjectIdentifier>> {
    let mut objects = Vec::new();
    let mut batch = limits.list_entries_per_request();
    let mut multiple_usable = true;
    let mut next = 1;
    while next <= count {
        if multiple_usable {
            let last = (next + batch - 1).min(count);
            match read_entries(client, instance, device, next, last, observe).await {
                Ok(Ok(entries)) => {
                    objects.extend(entries);
                    next = last + 1;
                    observe(Event::Listed {
                        read: objects.len(),
                        total: Some(count as usize),
                    });
                    continue;
                }
                Ok(Err(reason)) => bail!("cannot read the object list: {reason}"),
                Err(_) if batch > 1 => {
                    batch /= 2;
                    continue;
                }
                Err(_) => multiple_usable = false,
            }
        }
        objects.push(read_entry(client, instance, device, next, count, observe).await?);
        next += 1;
    }
    Ok(objects)
}

/// Entries `first` to `last` in one ReadPropertyMultiple. The outer error is the request
/// failing; the inner one an answer that holds no object.
async fn read_entries(
    client: &Client,
    instance: u32,
    device: ObjectIdentifier,
    first: u32,
    last: u32,
    observe: Observer<'_>,
) -> Result<Result<Vec<ObjectIdentifier>, String>, bacnet_types::error::Error> {
    let spec = ReadAccessSpecification {
        object_identifier: device,
        list_of_property_references: (first..=last)
            .map(|index| PropertyReference {
                property_identifier: PropertyIdentifier::OBJECT_LIST,
                property_array_index: Some(index),
            })
            .collect(),
    };
    let ack = timed(
        observe,
        || format!("ReadPropertyMultiple, object list {first} to {last}"),
        client.read_property_multiple_from_device(instance, vec![spec]),
    )
    .await?;
    let elements = ack
        .list_of_read_access_results
        .into_iter()
        .flat_map(|result| result.list_of_results);
    let mut objects = Vec::new();
    for element in elements {
        let index = element.property_array_index.unwrap_or_default();
        let value = match (element.property_value, element.error) {
            (Some(bytes), _) => decode_value(&bytes),
            (None, Some((class, code))) => {
                Err(Error::Remote(RemoteError::Error { class, code }).to_string())
            }
            (None, None) => Err("empty answer".into()),
        };
        match value {
            Ok(PropertyValue::ObjectIdentifier(object)) => objects.push(object),
            Ok(other) => {
                return Ok(Err(format!(
                    "entry {index} is {}, not an object",
                    value_kind_text(&other)
                )));
            }
            Err(reason) => return Ok(Err(format!("entry {index}: {reason}"))),
        }
    }
    let expected = (last - first + 1) as usize;
    if objects.len() != expected {
        return Ok(Err(format!(
            "asked for {expected} entries, got {}",
            objects.len()
        )));
    }
    Ok(Ok(objects))
}

async fn read_entry(
    client: &Client,
    instance: u32,
    device: ObjectIdentifier,
    index: u32,
    count: u32,
    observe: Observer<'_>,
) -> anyhow::Result<ObjectIdentifier> {
    let entry = timed(
        observe,
        || format!("ReadProperty object-list[{index}]"),
        read_value(
            client,
            instance,
            device,
            PropertyIdentifier::OBJECT_LIST,
            Some(index),
        ),
    )
    .await;
    match entry {
        Ok(PropertyValue::ObjectIdentifier(object)) => Ok(object),
        Ok(other) => bail!(
            "entry {index} of the object list is {}, not an object",
            value_kind_text(&other)
        ),
        Err(error) => bail!("cannot read entry {index} of {count} of the object list: {error}"),
    }
}

fn decode(bytes: &[u8]) -> Result<Vec<ObjectIdentifier>, String> {
    decode_values(bytes)?
        .into_iter()
        .map(|value| match value {
            PropertyValue::ObjectIdentifier(object) => Ok(object),
            other => Err(format!(
                "the object list holds {}, not objects",
                value_kind_text(&other)
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_list_decoding() {
        let bytes = [0xC4, 0x00, 0x00, 0x00, 0x01, 0xC4, 0x00, 0x40, 0x00, 0x02];
        let objects = decode(&bytes).unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].object_type(), ObjectType::ANALOG_INPUT);
        assert_eq!(objects[1].object_type(), ObjectType::ANALOG_OUTPUT);
        assert_eq!(objects[1].instance_number(), 2);
        assert!(decode(&[0x91, 0x00]).is_err());
    }

    /// What the device sent in words, not Rust's own notation: the message lands in the device
    /// list and in the tree.
    #[test]
    fn an_entry_that_is_no_object_says_what_came_instead() {
        assert_eq!(
            decode(&[0x21, 0x03]).unwrap_err(),
            "the object list holds the whole number 3, not objects"
        );
    }
}
