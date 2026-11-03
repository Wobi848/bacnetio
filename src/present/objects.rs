//! Objects by name: "Analog input 3" for showing, "ao:1" as typed on the command line.

use bacnet_types::enums::ObjectType;
use bacnet_types::primitives::ObjectIdentifier;

use super::sentence_case;

/// Short object type names as typed on the command line.
const TYPE_ABBREVIATIONS: &[(&str, ObjectType)] = &[
    ("ai", ObjectType::ANALOG_INPUT),
    ("ao", ObjectType::ANALOG_OUTPUT),
    ("av", ObjectType::ANALOG_VALUE),
    ("bi", ObjectType::BINARY_INPUT),
    ("bo", ObjectType::BINARY_OUTPUT),
    ("bv", ObjectType::BINARY_VALUE),
    ("msi", ObjectType::MULTI_STATE_INPUT),
    ("mso", ObjectType::MULTI_STATE_OUTPUT),
    ("msv", ObjectType::MULTI_STATE_VALUE),
];

/// "ao:1" or "1:1" (type number) to an object.
pub fn parse_object(text: &str) -> Result<ObjectIdentifier, String> {
    let invalid = || {
        format!(
            "{text} is not an object like ao:1 (types: ai ao av bi bo bv msi mso msv, or a number)"
        )
    };
    let (kind, instance) = text.split_once(':').ok_or_else(invalid)?;
    let kind = kind.trim().to_lowercase();
    let object_type = match TYPE_ABBREVIATIONS.iter().find(|(short, _)| *short == kind) {
        Some((_, object_type)) => *object_type,
        None => ObjectType::from_raw(kind.parse().map_err(|_| invalid())?),
    };
    let instance = instance.trim().parse().map_err(|_| invalid())?;
    ObjectIdentifier::new(object_type, instance).map_err(|error| error.to_string())
}

pub fn object_text(object: ObjectIdentifier) -> String {
    format!(
        "{} {}",
        object_type_text(object.object_type()),
        object.instance_number()
    )
}

/// "ANALOG_INPUT" → "Analog input".
pub fn object_type_text(object_type: ObjectType) -> String {
    match object_type.name() {
        Some(name) => sentence_case(name),
        None => format!("Type {}", object_type.to_raw()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The interface groups the tree and the type filter by type number and names the type itself
    /// (`objectTypes` in web/lang/*.js). A type missing there shows as a bare number, so every type
    /// the standard knows belongs in the table; the other languages follow English
    /// (tests/web/ui.test.js).
    #[test]
    fn every_object_type_has_a_name_in_the_interface() {
        let file = std::fs::read_to_string("web/lang/en.js").expect("en.js is readable");
        let table = file
            .split_once("objectTypes: {")
            .expect("en.js names the object types")
            .1
            .split_once('}')
            .expect("the table is closed")
            .0;
        let named: Vec<u32> = table
            .split(',')
            .filter_map(|entry| entry.split_once(':')?.0.trim().parse().ok())
            .collect();
        let missing: Vec<&str> = ObjectType::ALL_NAMED
            .iter()
            .filter(|(_, object_type)| !named.contains(&object_type.to_raw()))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(missing, Vec::<&str>::new());
    }
}
