//! A device's points and one object in depth.

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use bacnet_types::enums::{EngineeringUnits, ObjectType, PropertyIdentifier};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use serde::{Deserialize, Serialize};

use super::ApiError;
use super::json::{NoteView, ObjectView, PointView};
use super::session::Shared;
use crate::object;
use crate::points::ReadNote;
use crate::watch::{self, Source};
use crate::{alarms, bbmd, points, present, property_text, structure, write};

#[derive(Serialize)]
pub struct PointsView {
    points: Vec<PointView>,
    notes: Vec<NoteView<ReadNote>>,
}

pub async fn points(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<PointsView>, ApiError> {
    let client = state.client().await?;
    let reading = points::read_device(&client, device)
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    let view = PointsView {
        points: reading.points.iter().map(PointView::new).collect(),
        notes: reading.notes.into_iter().map(NoteView::new).collect(),
    };
    crate::locked(&state.readings).insert(device, reading.points);
    Ok(Json(view))
}

/// The points of the last reading without asking the device again, or null; so that a reloaded
/// page shows the open device at once.
pub async fn last_points(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Json<Option<serde_json::Value>> {
    if let Some(points) = crate::locked(&state.readings).get(&device) {
        let view = PointsView {
            points: points.iter().map(PointView::new).collect(),
            notes: Vec::new(),
        };
        return Json(Some(serde_json::to_value(view).expect("points are JSON")));
    }
    // after a restart: the points as they were remembered for the site
    let site = crate::locked(&state.site);
    Json(
        site.as_ref()
            .and_then(|site| site.points.get(&device.to_string()).cloned()),
    )
}

#[derive(Serialize)]
pub struct AlarmView {
    #[serde(flatten)]
    object: super::json::ObjectRef,
    /// The object's name from the last reading, if it was read.
    name: Option<String>,
    /// "offnormal", "fault", "high limit" …
    state: String,
    unacknowledged: bool,
    /// The device told what an acknowledgement has to refer to.
    can_acknowledge: bool,
}

/// The device's active alarms and faults (GetEventInformation).
pub async fn alarms(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<Vec<AlarmView>>, ApiError> {
    let client = state.client().await?;
    let found = alarms::read_alarms(&client, device)
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    let readings = crate::locked(&state.readings);
    let name = |object| {
        readings
            .get(&device)
            .and_then(|points| points.iter().find(|point| point.object == object))
            .and_then(|point| point.name.clone())
    };
    Ok(Json(
        found
            .into_iter()
            .map(|alarm| AlarmView {
                object: super::json::ObjectRef::new(alarm.object),
                name: name(alarm.object),
                state: alarms::state_text(alarm.state),
                unacknowledged: alarm.unacknowledged,
                can_acknowledge: alarm.timestamp.is_some(),
            })
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct PeerView {
    address: String,
    /// "255.255.255.255" means the broadcast goes to that BBMD itself.
    mask: String,
    /// It is the asked device's own row.
    own: bool,
}

#[derive(Serialize)]
pub struct ForeignView {
    address: String,
    ttl_s: u16,
    seconds_left: u16,
}

#[derive(Serialize)]
pub struct TablesView {
    /// The device gives its Broadcast Distribution Table, so it is a BBMD.
    is_bbmd: bool,
    bdt: Option<Vec<PeerView>>,
    fdt: Option<Vec<ForeignView>>,
    /// Why the Broadcast Distribution Table or the Foreign Device Table is missing.
    bdt_note: Option<NoteView<bbmd::TableNote>>,
    fdt_note: Option<NoteView<bbmd::TableNote>>,
}

/// The tables of a BBMD: which BBMDs it distributes broadcasts to (BDT) and which devices are
/// registered with it (FDT). Read only; nothing is written to the device.
pub async fn bbmd_tables(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<TablesView>, ApiError> {
    let address = crate::locked(&state.devices)
        .iter()
        .find(|found| found.instance == device)
        .map(|found| found.address)
        .ok_or_else(|| ApiError::conflict("search for devices first"))?;
    let address = address
        .ip()
        .ok_or_else(|| ApiError::conflict("only a device on BACnet/IP has BBMD tables"))?;
    let client = state.client().await?;
    let tables = bbmd::read_tables(&client, address).await;
    let peer = |peer: &bbmd::Peer| PeerView {
        address: present::address_text(peer.address),
        mask: peer.mask.to_string(),
        own: peer.address == address,
    };
    let foreign = |entry: &bbmd::Foreign| ForeignView {
        address: present::address_text(entry.address),
        ttl_s: entry.ttl_s,
        seconds_left: entry.seconds_left,
    };
    Ok(Json(TablesView {
        is_bbmd: tables.is_bbmd(),
        bdt: tables
            .broadcast
            .as_ref()
            .ok()
            .map(|peers| peers.iter().map(peer).collect()),
        fdt: tables
            .foreign
            .as_ref()
            .ok()
            .map(|entries| entries.iter().map(foreign).collect()),
        bdt_note: tables.broadcast.err().map(NoteView::new),
        fdt_note: tables.foreign.err().map(NoteView::new),
    }))
}

#[derive(Serialize)]
pub struct SubordinateView {
    #[serde(flatten)]
    object: super::json::ObjectRef,
    /// The device that holds the object, where the reference names one; else this device.
    device: Option<u32>,
    /// What the view says about this entry (Subordinate_Annotations).
    annotation: Option<String>,
    /// The object's name from the last reading, if it was read.
    name: Option<String>,
}

#[derive(Serialize)]
pub struct ViewView {
    #[serde(flatten)]
    object: super::json::ObjectRef,
    name: Option<String>,
    subordinates: Vec<SubordinateView>,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct StructureView {
    views: Vec<ViewView>,
}

/// The plant view the device carries itself: its Structured View objects (type 29) with what hangs
/// below each. An empty list means the device has none, and the UI groups by object names instead.
pub async fn structure(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<StructureView>, ApiError> {
    let (views, names) = {
        let readings = crate::locked(&state.readings);
        let points = readings
            .get(&device)
            .ok_or_else(|| ApiError::conflict("read the device's points first"))?;
        let names = points
            .iter()
            .filter_map(|point| point.name.clone().map(|name| (point.object, name)))
            .collect::<HashMap<_, _>>();
        (
            structure::views_of(points.iter().map(|point| point.object)),
            names,
        )
    };
    if views.is_empty() {
        return Ok(Json(StructureView { views: Vec::new() }));
    }
    // a remembered site has no connection yet: the tree groups by names until the next search
    let Ok(client) = state.client().await else {
        return Ok(Json(StructureView { views: Vec::new() }));
    };
    let read = structure::read_views(&client, device, &views).await;
    let view = |view: &structure::View| ViewView {
        object: super::json::ObjectRef::new(view.object),
        name: names.get(&view.object).cloned(),
        subordinates: view
            .subordinates
            .iter()
            .map(|entry| SubordinateView {
                object: super::json::ObjectRef::new(entry.object),
                device: entry.device,
                annotation: entry.annotation.clone(),
                name: entry
                    .device
                    .is_none()
                    .then(|| names.get(&entry.object).cloned())
                    .flatten(),
            })
            .collect(),
        error: view.error.clone(),
    };
    Ok(Json(StructureView {
        views: read.iter().map(view).collect(),
    }))
}

pub async fn object(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<Json<ObjectView>, ApiError> {
    let _first = state.precedence.first();
    let object = object_id(object_type, instance)?;
    Ok(Json(object_view(&state, device, object).await?))
}

#[derive(Serialize)]
pub struct ValueView {
    /// Without unit: "21.4", "active", "state 2".
    value: String,
    /// Status flags that are set, in words; `None` when not known.
    status: Option<String>,
    /// `cov` when the device reports changes itself, `read` when it was asked.
    source: &'static str,
}

/// The present value, live: from the device's own reports (COV) where it sends them, else read.
#[derive(Deserialize)]
pub struct ValueQuery {
    /// "read" asks the device each time instead of following its COV reports.
    mode: Option<String>,
}

pub async fn value(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
    Query(query): Query<ValueQuery>,
) -> Result<Json<ValueView>, ApiError> {
    let _first = state.precedence.first();
    let object = object_id(object_type, instance)?;
    let client = state.client().await?;
    let found = if query.mode.as_deref() == Some("read") {
        watch::read_now(&client, device, object).await
    } else {
        watch::current_value(&state.watch, &client, device, object).await
    };
    let (live, source) = found.map_err(|error| ApiError::device(error.to_string()))?;
    Ok(Json(ValueView {
        value: present::plain_text(object.object_type(), &live.value),
        status: live.status.map(|status| present::status_text(Some(status))),
        source: match source {
            Source::Cov => "cov",
            Source::Read => "read",
        },
    }))
}

#[derive(Serialize)]
pub struct PropertyView {
    /// "present-value"
    property: String,
    value: Option<String>,
    error: Option<String>,
    /// What kind of value it is: "real", "text", …
    data_type: Option<&'static str>,
    /// Each entry in words, for a list or array: event time stamps, message texts.
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Vec<String>>,
    /// Each bit, first first, for a bit string: status flags, event enable, limit enable.
    #[serde(skip_serializing_if = "Option::is_none")]
    bits: Option<Vec<bool>>,
    /// The property's number, to write it.
    id: u32,
    /// It holds a plain value and is not reserved: the device decides whether it takes a write.
    writable: bool,
}

fn data_type(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Null => "null",
        PropertyValue::Boolean(_) => "boolean",
        PropertyValue::Unsigned(_) => "unsigned",
        PropertyValue::Signed(_) => "signed",
        PropertyValue::Real(_) => "real",
        PropertyValue::Double(_) => "double",
        PropertyValue::CharacterString(_) => "text",
        PropertyValue::OctetString(_) => "octets",
        PropertyValue::BitString { .. } => "bits",
        PropertyValue::Enumerated(_) => "enumerated",
        PropertyValue::Date(_) => "date",
        PropertyValue::Time(_) => "time",
        PropertyValue::ObjectIdentifier(_) => "object",
        _ => "other",
    }
}

/// A list or array: several values, or one list value.
fn is_list(values: &[PropertyValue]) -> bool {
    matches!(values, [_, _, ..] | [PropertyValue::List(_)])
}

/// Every property of the object, for the technical details.
pub async fn properties(
    State(state): State<Shared>,
    Path((device, object_type, instance)): Path<(u32, u32, u32)>,
) -> Result<Json<Vec<PropertyView>>, ApiError> {
    let _first = state.precedence.first();
    let object = object_id(object_type, instance)?;
    let client = state.client().await?;
    let readings = object::read_properties(&client, device, object).await;
    let units = readings
        .iter()
        .find_map(|reading| match (reading.property, &reading.values) {
            (PropertyIdentifier::UNITS, Ok(values)) => match values.first() {
                Some(PropertyValue::Enumerated(raw)) => Some(EngineeringUnits::from_raw(*raw)),
                _ => None,
            },
            _ => None,
        });
    Ok(Json(
        readings
            .iter()
            .map(|reading| {
                let (value, error) = match &reading.values {
                    Ok(values) => (
                        Some(property_text::property_text(
                            object.object_type(),
                            reading.property,
                            values,
                            units,
                            &reading.encoded,
                            crate::connection::link(&client),
                        )),
                        None,
                    ),
                    Err(reason) => (None, Some(reason.clone())),
                };
                let values = reading.values.as_deref().unwrap_or_default();
                PropertyView {
                    property: property_text::property_name(reading.property),
                    value,
                    error,
                    items: is_list(values).then(|| {
                        property_text::property_items(
                            object.object_type(),
                            reading.property,
                            values,
                            units,
                        )
                    }),
                    bits: match values {
                        [PropertyValue::BitString { unused_bits, data }] => {
                            Some(crate::present::bits(*unused_bits, data))
                        }
                        _ => None,
                    },
                    data_type: reading
                        .values
                        .as_ref()
                        .ok()
                        .and_then(|values| values.first())
                        .map(data_type),
                    id: reading.property.to_raw(),
                    writable: !write::NOT_WRITABLE.contains(&reading.property)
                        && matches!(
                            reading.values.as_deref(),
                            Ok([PropertyValue::Boolean(_)
                                | PropertyValue::Unsigned(_)
                                | PropertyValue::Signed(_)
                                | PropertyValue::Real(_)
                                | PropertyValue::Double(_)
                                | PropertyValue::CharacterString(_)
                                | PropertyValue::Enumerated(_)])
                        ),
                }
            })
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct WinnerView {
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
    /// The priority that wins now; objects without a set priority are left out.
    winner: usize,
}

/// Which priority wins on each object that can have one, read one object after the other.
pub async fn priorities(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<Vec<WinnerView>>, ApiError> {
    let objects: Vec<ObjectIdentifier> = crate::locked(&state.readings)
        .get(&device)
        .ok_or_else(|| ApiError::conflict("read the device's points first"))?
        .iter()
        .map(|point| point.object)
        .filter(|object| object::commandable_kind(object.object_type()))
        .collect();
    let client = state.client().await?;
    let mut winners = Vec::new();
    for object in objects {
        let Ok(Some(slots)) = object::read_priorities(&client, device, object).await else {
            continue;
        };
        if let Some(index) = slots.iter().position(Option::is_some) {
            winners.push(WinnerView {
                object_type: object.object_type().to_raw(),
                instance: object.instance_number(),
                winner: index + 1,
            });
        }
    }
    Ok(Json(winners))
}

/// More rows than a screen holds; a request for all of a large device would keep it busy.
const MOST_LIVE_OBJECTS: usize = 200;

#[derive(Deserialize)]
pub struct LiveRequest {
    objects: Vec<LiveObject>,
}

#[derive(Deserialize)]
pub struct LiveObject {
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
}

/// The objects on screen read again, which also updates what the problems and exports know.
pub async fn live_values(
    State(state): State<Shared>,
    Path(device): Path<u32>,
    Json(request): Json<LiveRequest>,
) -> Result<Json<Vec<PointView>>, ApiError> {
    if request.objects.len() > MOST_LIVE_OBJECTS {
        return Err(ApiError::bad_request(format!(
            "at most {MOST_LIVE_OBJECTS} objects at a time"
        )));
    }
    let objects = request
        .objects
        .iter()
        .map(|item| object_id(item.object_type, item.instance))
        .collect::<Result<Vec<_>, _>>()?;
    let client = state.client().await?;
    let fresh = points::read_again(&client, device, objects).await;
    if let Some(known) = crate::locked(&state.readings).get_mut(&device) {
        for point in fresh.iter().filter(|point| point.error.is_none()) {
            if let Some(listed) = known
                .iter_mut()
                .find(|listed| listed.object == point.object)
            {
                listed.value = point.value.clone();
                listed.status = point.status;
            }
        }
    }
    Ok(Json(fresh.iter().map(PointView::new).collect()))
}

pub(super) async fn object_view(
    state: &Shared,
    device: u32,
    object: ObjectIdentifier,
) -> Result<ObjectView, ApiError> {
    let client = state.client().await?;
    let detail = object::read_object(&client, device, object).await;
    // what was just read is newer than the last reading of the device, which the problems, the
    // search and the exports use
    if detail.point.error.is_none()
        && let Some(listed) = crate::locked(&state.readings)
            .get_mut(&device)
            .and_then(|points| points.iter_mut().find(|point| point.object == object))
    {
        listed.value = detail.point.value.clone();
        listed.status = detail.point.status;
        listed.name = detail.point.name.clone();
        listed.description = detail.point.description.clone();
    }
    let mut journal = crate::locked(&state.journal);
    if let Some(slots) = &detail.priority_array {
        journal.compare_with_device(device, object, slots);
    }
    let mine: Vec<(u8, PropertyValue)> = journal
        .held()
        .iter()
        .filter(|entry| entry.device == device && entry.object == object)
        .filter_map(|entry| match (&entry.action, entry.priority) {
            (write::Action::Write(value), Some(priority)) => Some((priority, value.clone())),
            _ => None,
        })
        .collect();
    Ok(ObjectView::new(&detail, &mine))
}

pub(super) fn object_id(object_type: u32, instance: u32) -> Result<ObjectIdentifier, ApiError> {
    ObjectIdentifier::new(ObjectType::from_raw(object_type), instance)
        .map_err(|error| ApiError::bad_request(error.to_string()))
}
