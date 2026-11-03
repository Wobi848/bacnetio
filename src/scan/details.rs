//! Name, vendor and model of found devices.

use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use super::Details;
use crate::connection::{Client, read_value};
use crate::present::value_kind_text;

pub(super) async fn device_details(client: &Client, instance: u32) -> Details {
    let mut details = Details::default();
    let read = |property| read_text(client, instance, property);
    for (property, field) in [
        (PropertyIdentifier::OBJECT_NAME, &mut details.name),
        (PropertyIdentifier::VENDOR_NAME, &mut details.vendor),
        (PropertyIdentifier::MODEL_NAME, &mut details.model),
    ] {
        match read(property).await {
            Ok(text) => *field = Some(text),
            Err(error) => {
                details.error.get_or_insert(error);
            }
        }
    }
    details
}

async fn read_text(
    client: &Client,
    instance: u32,
    property: PropertyIdentifier,
) -> Result<String, String> {
    let device = ObjectIdentifier::new(ObjectType::DEVICE, instance).map_err(|e| e.to_string())?;
    match read_value(client, instance, device, property, None).await {
        Ok(PropertyValue::CharacterString(text)) => Ok(text),
        Ok(other) => Err(format!(
            "the device answered {}, not a text",
            value_kind_text(&other)
        )),
        Err(error) => Err(error.to_string()),
    }
}
