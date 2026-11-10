//! What the user typed, as BACnet values: a present value by the object's type, any property by
//! the kind it holds now, and a date and time for the device's clock. Every check of a typed
//! value is here, so the API and the CLI refuse the same things.

use bacnet_types::enums::ObjectType;
use bacnet_types::primitives::{Date, PropertyValue, Time};

/// A typed value from what was typed, of the kind the property holds now (`data_type` in the API).
pub fn parse_property_value(kind: &str, text: &str) -> Result<PropertyValue, String> {
    let text = text.trim();
    let invalid = |what: &str| format!("\"{text}\" is not {what}");
    match kind {
        "boolean" => match text {
            "true" | "1" => Ok(PropertyValue::Boolean(true)),
            "false" | "0" => Ok(PropertyValue::Boolean(false)),
            _ => Err(invalid("true or false")),
        },
        "text" => Ok(PropertyValue::CharacterString(text.to_string())),
        // "NaN" and "inf" parse as numbers, but no limit or increment should ever be one
        "real" => text
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|value: &f32| value.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(|| invalid("a number")),
        "double" => text
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|value: &f64| value.is_finite())
            .map(PropertyValue::Double)
            .ok_or_else(|| invalid("a number")),
        "unsigned" => text
            .parse()
            .map(PropertyValue::Unsigned)
            .map_err(|_| invalid("a whole number")),
        "signed" => text
            .parse()
            .map(PropertyValue::Signed)
            .map_err(|_| invalid("a whole number")),
        "enumerated" => text
            .parse()
            .map(PropertyValue::Enumerated)
            .map_err(|_| invalid("the number of a choice")),
        other => Err(format!("a value of kind {other} cannot be written here")),
    }
}

/// "2026-11-01" and "09:30:05" with the weekday (1 = Monday) to BACnet date and time.
pub fn parse_clock(date: &str, time: &str, weekday: u8) -> Result<(Date, Time), String> {
    let invalid = || format!("{date} {time} is not a date and time like 2026-11-01 09:30:05");
    let numbers = |text: &str, separator: char| -> Result<Vec<u32>, String> {
        text.split(separator)
            .map(|part| part.trim().parse().map_err(|_| invalid()))
            .collect()
    };
    let (Ok([year, month, day]), Ok([hour, minute, second])) = (
        <[u32; 3]>::try_from(numbers(date, '-')?),
        <[u32; 3]>::try_from(numbers(time, ':')?),
    ) else {
        return Err(invalid());
    };
    let fits = (1900..=2154).contains(&year)
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && (1..=7).contains(&weekday)
        && hour < 24
        && minute < 60
        && second < 60;
    if !fits {
        return Err(invalid());
    }
    let date = Date {
        year: (year - 1900) as u8,
        month: month as u8,
        day: day as u8,
        day_of_week: weekday,
    };
    let time = Time {
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
        hundredths: 0,
    };
    Ok((date, time))
}

/// The value typed by the user, in the data type the object expects.
pub fn parse_value(object_type: ObjectType, text: &str) -> Result<PropertyValue, String> {
    let text = text.trim();
    match object_type {
        ObjectType::ANALOG_INPUT | ObjectType::ANALOG_OUTPUT | ObjectType::ANALOG_VALUE => text
            .replace(',', ".")
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(|| format!("\"{text}\" is not a number")),
        ObjectType::BINARY_INPUT | ObjectType::BINARY_OUTPUT | ObjectType::BINARY_VALUE => {
            match text.to_lowercase().as_str() {
                "active" | "on" | "1" | "true" => Ok(PropertyValue::Enumerated(1)),
                "inactive" | "off" | "0" | "false" => Ok(PropertyValue::Enumerated(0)),
                _ => Err(format!("\"{text}\" is not active or inactive")),
            }
        }
        ObjectType::MULTI_STATE_INPUT
        | ObjectType::MULTI_STATE_OUTPUT
        | ObjectType::MULTI_STATE_VALUE => text
            .parse::<u64>()
            .ok()
            .filter(|state| *state >= 1)
            .map(PropertyValue::Unsigned)
            .ok_or_else(|| format!("\"{text}\" is not a state number (1, 2, ...)")),
        ObjectType::LIGHTING_OUTPUT => text
            .replace(',', ".")
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite())
            .map(PropertyValue::Real)
            .ok_or_else(|| format!("\"{text}\" is not a number")),
        ObjectType::LARGE_ANALOG_VALUE => text
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(PropertyValue::Double)
            .ok_or_else(|| format!("\"{text}\" is not a number")),
        ObjectType::INTEGER_VALUE => text
            .parse::<i32>()
            .map(PropertyValue::Signed)
            .map_err(|_| format!("\"{text}\" is not a whole number")),
        ObjectType::POSITIVE_INTEGER_VALUE => text
            .parse::<u64>()
            .map(PropertyValue::Unsigned)
            .map_err(|_| format!("\"{text}\" is not a positive whole number")),
        ObjectType::CHARACTERSTRING_VALUE => Ok(PropertyValue::CharacterString(text.to_string())),
        ObjectType::BINARY_LIGHTING_OUTPUT => text
            .parse::<u32>()
            .ok()
            .filter(|state| *state <= 6)
            .map(PropertyValue::Enumerated)
            .ok_or_else(|| format!("\"{text}\" is not a lighting state (0 off, 1 on, ...)")),
        _ => Err("objects of this type cannot be written yet".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_other_commandable_types_take_a_value_of_their_kind() {
        let parsed = |object_type, text| parse_value(object_type, text);
        assert_eq!(
            parsed(ObjectType::INTEGER_VALUE, "-5"),
            Ok(PropertyValue::Signed(-5))
        );
        assert!(parsed(ObjectType::INTEGER_VALUE, "1.5").is_err());
        assert_eq!(
            parsed(ObjectType::POSITIVE_INTEGER_VALUE, "7"),
            Ok(PropertyValue::Unsigned(7))
        );
        assert!(parsed(ObjectType::POSITIVE_INTEGER_VALUE, "-1").is_err());
        assert_eq!(
            parsed(ObjectType::LARGE_ANALOG_VALUE, "1234567,5"),
            Ok(PropertyValue::Double(1_234_567.5))
        );
        assert_eq!(
            parsed(ObjectType::LIGHTING_OUTPUT, "80"),
            Ok(PropertyValue::Real(80.0))
        );
        assert_eq!(
            parsed(ObjectType::CHARACTERSTRING_VALUE, " Winter "),
            Ok(PropertyValue::CharacterString("Winter".to_string()))
        );
        assert_eq!(
            parsed(ObjectType::BINARY_LIGHTING_OUTPUT, "1"),
            Ok(PropertyValue::Enumerated(1))
        );
        assert!(parsed(ObjectType::BINARY_LIGHTING_OUTPUT, "9").is_err());
        assert!(parsed(ObjectType::DEVICE, "1").is_err());
    }

    #[test]
    fn values_are_typed_by_object() {
        assert_eq!(
            parse_value(ObjectType::ANALOG_VALUE, " 21,5 "),
            Ok(PropertyValue::Real(21.5))
        );
        assert_eq!(
            parse_value(ObjectType::BINARY_OUTPUT, "Active"),
            Ok(PropertyValue::Enumerated(1))
        );
        assert_eq!(
            parse_value(ObjectType::MULTI_STATE_VALUE, "2"),
            Ok(PropertyValue::Unsigned(2))
        );
        assert!(parse_value(ObjectType::MULTI_STATE_VALUE, "0").is_err());
        assert!(parse_value(ObjectType::ANALOG_VALUE, "warm").is_err());
        assert!(parse_value(ObjectType::DEVICE, "1").is_err());
    }

    #[test]
    fn clock_from_text() {
        let (date, time) = parse_clock("2026-11-01", "09:30:05", 7).unwrap();
        assert_eq!(
            (date.year, date.month, date.day, date.day_of_week),
            (126, 11, 1, 7)
        );
        assert_eq!((time.hour, time.minute, time.second), (9, 30, 5));
        assert!(parse_clock("2026-13-01", "09:30:05", 7).is_err());
        assert!(parse_clock("2026-11-01", "9:30", 7).is_err());
    }
}
