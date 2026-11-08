//! Reading a device's data points: the object list, then name, value, unit and status of every
//! object. Falls back to simpler requests where a device cannot do more, and remembers that for
//! the rest of the device (concept section 6).

use std::fmt;
use std::time::{Duration, Instant};

use bacnet_types::constructed::{PropertyReference, ReadAccessSpecification};
use bacnet_types::enums::{
    EngineeringUnits, ErrorClass, ErrorCode, ObjectType, PropertyIdentifier,
};
use bacnet_types::error::{Error, RemoteError};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use serde::Serialize;

use crate::connection::{Client, decode_value, decode_values, is_timeout};
use crate::limits::{self, Limits};
use crate::object_list;
use crate::observe::{Event, Observer, Request, timed};
use crate::precedence::Precedence;
use crate::present;
use crate::property_text;

const PROPERTIES: [PropertyIdentifier; 5] = [
    PropertyIdentifier::OBJECT_NAME,
    PropertyIdentifier::PRESENT_VALUE,
    PropertyIdentifier::UNITS,
    PropertyIdentifier::STATUS_FLAGS,
    PropertyIdentifier::DESCRIPTION,
];
/// The objects a plant alarms on, whose notification class the EDE file names. Asked of these only:
/// on a device without ReadPropertyMultiple every property costs a request of its own.
const REPORTING: [ObjectType; 10] = [
    ObjectType::ANALOG_INPUT,
    ObjectType::ANALOG_OUTPUT,
    ObjectType::ANALOG_VALUE,
    ObjectType::BINARY_INPUT,
    ObjectType::BINARY_OUTPUT,
    ObjectType::BINARY_VALUE,
    ObjectType::LOOP,
    ObjectType::MULTI_STATE_INPUT,
    ObjectType::MULTI_STATE_OUTPUT,
    ObjectType::MULTI_STATE_VALUE,
];
/// Objects without any answer in a row before the device counts as gone.
const GIVE_UP_AFTER_SILENT_OBJECTS: usize = 3;
/// Refusals of ReadPropertyMultiple for a single object before it is no longer tried.
const MULTIPLE_REFUSALS_ALLOWED: usize = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusFlags {
    pub in_alarm: bool,
    pub fault: bool,
    pub overridden: bool,
    pub out_of_service: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub object: ObjectIdentifier,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value: Option<PropertyValue>,
    pub units: Option<EngineeringUnits>,
    pub status: Option<StatusFlags>,
    /// Texts of a multi-state object's states, from state 1 on.
    pub state_texts: Vec<String>,
    /// A binary object's Inactive_Text and Active_Text, in this order; empty when it has none.
    pub binary_texts: Vec<String>,
    /// The Notification_Class of an object that reports alarms by itself.
    pub notification_class: Option<u32>,
    /// The first thing that went wrong reading this object.
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct DeviceReading {
    pub points: Vec<Point>,
    /// How the device had to be read.
    pub notes: Vec<ReadNote>,
}

/// How a device had to be read. Shown in words through `Display`; the UI translates it by its
/// code.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ReadNote {
    ObjectListEntryByEntry,
    NoReadPropertyMultiple,
    StoppedAnswering { unread: usize },
    StoppedByUser { unread: usize },
}

impl fmt::Display for ReadNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadNote::ObjectListEntryByEntry => write!(
                f,
                "The device cannot send its object list in one piece; it was read in parts."
            ),
            ReadNote::NoReadPropertyMultiple => write!(
                f,
                "The device does not handle ReadPropertyMultiple; values were read one by one."
            ),
            ReadNote::StoppedAnswering { unread } => write!(
                f,
                "The device stopped answering; {unread} objects were not read."
            ),
            ReadNote::StoppedByUser { unread } => {
                write!(f, "Reading was stopped; {unread} objects were not read.")
            }
        }
    }
}

pub async fn read_device(client: &Client, instance: u32) -> anyhow::Result<DeviceReading> {
    read_device_observed(client, instance, None, &|_| {}, &Precedence::default()).await
}

/// Like [`read_device`], reporting each request and how far it is to `observe`. Request sizes
/// follow the limits the device tells (limits.rs). `known_list` is an object list the search read
/// already, and whether it took parts; it is not asked for again. Between two requests the reading
/// gives way to what a user waits for (`precedence`).
pub async fn read_device_observed(
    client: &Client,
    instance: u32,
    known_list: Option<(Vec<ObjectIdentifier>, bool)>,
    observe: Observer<'_>,
    precedence: &Precedence,
) -> anyhow::Result<DeviceReading> {
    let started = Instant::now();
    let told = limits::read_limits(client, instance).await;
    observe(Event::Request(Request {
        what: "ReadPropertyMultiple, max-apdu-length-accepted and segmentation-supported".into(),
        took: started.elapsed(),
        failed: told
            .is_none()
            .then(|| "not told; assuming 1476 bytes with segmentation".into()),
    }));
    let limits = told.unwrap_or(Limits::ASSUMED);
    let mut notes = Vec::new();
    let (objects, in_parts) = match known_list {
        Some(known) => {
            observe(Event::Request(Request {
                what: format!("Object list from the search, {} entries", known.0.len()),
                took: Duration::ZERO,
                failed: None,
            }));
            known
        }
        None => object_list::read(client, instance, limits, observe).await?,
    };
    if in_parts {
        notes.push(ReadNote::ObjectListEntryByEntry);
    }
    let mut points: Vec<Point> = objects.into_iter().map(Point::new).collect();
    observe(Event::Values {
        read: 0,
        points: &points,
    });
    let batch = limits.points_per_request();
    read_properties(
        client,
        instance,
        &mut points,
        &mut notes,
        batch,
        observe,
        precedence,
    )
    .await;
    Ok(DeviceReading { points, notes })
}

/// These objects read again, name, value and state, sized to the device's limits. For values live
/// in the list, and only for what is on screen.
pub async fn read_again(
    client: &Client,
    instance: u32,
    objects: Vec<ObjectIdentifier>,
) -> Vec<Point> {
    let limits = limits::read_limits(client, instance)
        .await
        .unwrap_or(Limits::ASSUMED);
    let mut points: Vec<Point> = objects.into_iter().map(Point::new).collect();
    let mut notes = Vec::new();
    let batch = limits.points_per_request();
    read_properties(
        client,
        instance,
        &mut points,
        &mut notes,
        batch,
        &|_| {},
        &Precedence::default(),
    )
    .await;
    points
}

async fn read_properties(
    client: &Client,
    instance: u32,
    points: &mut [Point],
    notes: &mut Vec<ReadNote>,
    mut batch: usize,
    observe: Observer<'_>,
    precedence: &Precedence,
) {
    let mut multiple_usable = true;
    let mut multiple_refusals = 0;
    let mut silent_objects = 0;
    let mut next = 0;
    while next < points.len() {
        precedence.turn().await;
        if silent_objects >= GIVE_UP_AFTER_SILENT_OBJECTS {
            let left = points.len() - next;
            for point in &mut points[next..] {
                point
                    .error
                    .get_or_insert("not read: the device stopped answering".into());
            }
            notes.push(ReadNote::StoppedAnswering { unread: left });
            return;
        }
        if multiple_usable {
            let end = (next + batch).min(points.len());
            match read_multiple(client, instance, &mut points[next..end], observe).await {
                Ok(()) => {
                    next = end;
                    silent_objects = 0;
                    observe(Event::Values { read: next, points });
                    continue;
                }
                Err(_) if batch > 1 => {
                    batch /= 2;
                    continue;
                }
                Err(_) => {
                    multiple_refusals += 1;
                    if multiple_refusals >= MULTIPLE_REFUSALS_ALLOWED {
                        multiple_usable = false;
                        notes.push(ReadNote::NoReadPropertyMultiple);
                    }
                }
            }
        }
        let answered = read_single(client, instance, &mut points[next], observe).await;
        silent_objects = if answered { 0 } else { silent_objects + 1 };
        next += 1;
        observe(Event::Values { read: next, points });
    }
}

/// All properties of all `points` in one request.
async fn read_multiple(
    client: &Client,
    instance: u32,
    points: &mut [Point],
    observe: Observer<'_>,
) -> Result<(), Error> {
    let specs = points
        .iter()
        .map(|point| ReadAccessSpecification {
            object_identifier: point.object,
            list_of_property_references: point
                .properties()
                .into_iter()
                .map(|property_identifier| PropertyReference {
                    property_identifier,
                    property_array_index: None,
                })
                .collect(),
        })
        .collect();
    let count = points.len();
    let ack = timed(
        observe,
        || format!("ReadPropertyMultiple, {count} objects"),
        client.read_property_multiple_from_device(instance, specs),
    )
    .await?;
    for result in ack.list_of_read_access_results {
        let Some(point) = points
            .iter_mut()
            .find(|p| p.object == result.object_identifier)
        else {
            continue;
        };
        for element in result.list_of_results {
            let property = element.property_identifier;
            match (element.property_value, element.error) {
                (Some(bytes), _) => point.take(property, &bytes),
                (None, Some((class, code))) => point.refused(property, class, code),
                (None, None) => point.fail(property, "empty answer"),
            }
        }
    }
    Ok(())
}

/// Each property of one point in its own request. False if the device did not answer at all.
pub(crate) async fn read_single(
    client: &Client,
    instance: u32,
    point: &mut Point,
    observe: Observer<'_>,
) -> bool {
    for property in point.properties() {
        let object = point.object;
        let answer = timed(
            observe,
            || {
                let name = property.name().unwrap_or("property").to_lowercase();
                format!("ReadProperty {name} of {}", present::object_text(object))
            },
            client.read_property_from_device(instance, object, property, None),
        )
        .await;
        match answer {
            Ok(ack) => point.take(property, &ack.property_value),
            Err(error) if is_timeout(&error) => {
                point.fail(property, &error.to_string());
                // the first property already tells whether the device is there at all
                if property == PROPERTIES[0] {
                    return false;
                }
            }
            Err(Error::Remote(RemoteError::Error { class, code })) => {
                point.refused(property, class, code)
            }
            Err(error) => point.fail(property, &error.to_string()),
        }
    }
    true
}

impl Point {
    pub fn new(object: ObjectIdentifier) -> Self {
        Point {
            object,
            name: None,
            description: None,
            value: None,
            units: None,
            status: None,
            state_texts: Vec::new(),
            binary_texts: Vec::new(),
            notification_class: None,
            error: None,
        }
    }

    /// What is read of this object: multi-state and binary objects also name their states, and
    /// objects that alarm their notification class.
    fn properties(&self) -> Vec<PropertyIdentifier> {
        let kind = self.object.object_type();
        let mut properties = PROPERTIES.to_vec();
        if present::is_multistate(kind) {
            properties.push(PropertyIdentifier::STATE_TEXT);
        }
        if present::is_binary(kind) {
            properties.extend([
                PropertyIdentifier::INACTIVE_TEXT,
                PropertyIdentifier::ACTIVE_TEXT,
            ]);
        }
        if REPORTING.contains(&kind) {
            properties.push(PropertyIdentifier::NOTIFICATION_CLASS);
        }
        properties
    }

    /// An answer for one property, as the device encoded it.
    fn take(&mut self, property: PropertyIdentifier, bytes: &[u8]) {
        if property == PropertyIdentifier::STATE_TEXT {
            match decode_values(bytes) {
                Ok(values) => self.state_texts = values.into_iter().map(state_text).collect(),
                Err(reason) => self.fail(property, &reason),
            }
            return;
        }
        match decode_value(bytes) {
            Ok(value) => self.apply(property, value),
            Err(reason) => self.fail(property, &reason),
        }
    }

    fn apply(&mut self, property: PropertyIdentifier, value: PropertyValue) {
        match (property, value) {
            (PropertyIdentifier::OBJECT_NAME, PropertyValue::CharacterString(text)) => {
                self.name = Some(text)
            }
            (PropertyIdentifier::DESCRIPTION, PropertyValue::CharacterString(text)) => {
                self.description = Some(text)
            }
            (PropertyIdentifier::PRESENT_VALUE, value) => self.value = Some(value),
            (PropertyIdentifier::UNITS, PropertyValue::Enumerated(raw)) => {
                self.units = Some(EngineeringUnits::from_raw(raw))
            }
            (PropertyIdentifier::STATUS_FLAGS, PropertyValue::BitString { data, .. }) => {
                self.status = Some(StatusFlags::from_bits(&data))
            }
            (PropertyIdentifier::INACTIVE_TEXT, PropertyValue::CharacterString(text)) => {
                self.binary_text(0, text)
            }
            (PropertyIdentifier::ACTIVE_TEXT, PropertyValue::CharacterString(text)) => {
                self.binary_text(1, text)
            }
            (PropertyIdentifier::NOTIFICATION_CLASS, PropertyValue::Unsigned(class)) => {
                self.notification_class = u32::try_from(class).ok()
            }
            (property, other) => self.fail(
                property,
                &format!(
                    "the device answered {}, which does not fit here",
                    present::value_kind_text(&other)
                ),
            ),
        }
    }

    /// Inactive (0) or active (1) text; the other keeps its place, empty until it comes.
    fn binary_text(&mut self, index: usize, text: String) {
        self.binary_texts.resize(2, String::new());
        self.binary_texts[index] = text;
    }

    /// The device answered with an error. A property the object does not have is normal.
    fn refused(&mut self, property: PropertyIdentifier, class: ErrorClass, code: ErrorCode) {
        if code != ErrorCode::UNKNOWN_PROPERTY {
            let error = Error::Remote(RemoteError::Error { class, code });
            self.fail(property, &error.to_string());
        }
    }

    fn fail(&mut self, property: PropertyIdentifier, reason: &str) {
        let property = property_text::property_name(property);
        self.error.get_or_insert(format!("{property}: {reason}"));
    }
}

/// A state text; a state without text keeps its place in the list.
fn state_text(value: PropertyValue) -> String {
    match value {
        PropertyValue::CharacterString(text) => text,
        _ => String::new(),
    }
}

impl StatusFlags {
    /// Clause 12.50: in-alarm, fault, overridden, out-of-service, from the first bit on.
    pub(crate) fn from_bits(data: &[u8]) -> Self {
        let first = data.first().copied().unwrap_or(0);
        StatusFlags {
            in_alarm: first & 0x80 != 0,
            fault: first & 0x40 != 0,
            overridden: first & 0x20 != 0,
            out_of_service: first & 0x10 != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::enums::ObjectType;

    fn analog_input(instance: u32) -> Point {
        Point::new(ObjectIdentifier::new(ObjectType::ANALOG_INPUT, instance).unwrap())
    }

    #[test]
    fn status_flags_from_first_byte() {
        assert_eq!(StatusFlags::from_bits(&[0x00]), StatusFlags::default());
        let flags = StatusFlags::from_bits(&[0x50]);
        assert!(flags.fault && flags.out_of_service);
        assert!(!flags.in_alarm && !flags.overridden);
        assert_eq!(StatusFlags::from_bits(&[]), StatusFlags::default());
    }

    #[test]
    fn missing_property_is_not_an_error() {
        let mut point = analog_input(1);
        point.refused(
            PropertyIdentifier::UNITS,
            ErrorClass::PROPERTY,
            ErrorCode::UNKNOWN_PROPERTY,
        );
        assert_eq!(point.error, None);
        point.refused(
            PropertyIdentifier::PRESENT_VALUE,
            ErrorClass::OBJECT,
            ErrorCode::UNKNOWN_OBJECT,
        );
        assert!(point.error.unwrap().starts_with("present-value:"));
    }

    #[test]
    fn values_land_in_their_fields() {
        let mut point = analog_input(1);
        point.apply(
            PropertyIdentifier::OBJECT_NAME,
            PropertyValue::CharacterString("Supply air".into()),
        );
        point.apply(PropertyIdentifier::PRESENT_VALUE, PropertyValue::Real(21.5));
        point.apply(
            PropertyIdentifier::UNITS,
            PropertyValue::Enumerated(EngineeringUnits::DEGREES_CELSIUS.to_raw()),
        );
        assert_eq!(point.name.as_deref(), Some("Supply air"));
        assert_eq!(point.value, Some(PropertyValue::Real(21.5)));
        assert_eq!(point.units, Some(EngineeringUnits::DEGREES_CELSIUS));
        assert_eq!(point.error, None);
    }

    /// A sentence a technician can act on, with the property and the answer in words.
    #[test]
    fn wrong_value_type_is_reported() {
        let mut point = analog_input(1);
        point.apply(PropertyIdentifier::OBJECT_NAME, PropertyValue::Real(1.0));
        assert_eq!(
            point.error.unwrap(),
            "object-name: the device answered the number 1, which does not fit here"
        );
    }
}
