//! The remembered site back as working data, so that exports and the views of a device work after
//! a restart as they do after reading it. The file holds what the interface showed: values as text,
//! which is all an export of remembered data can give.

use std::collections::HashMap;
use std::net::SocketAddrV4;
use std::sync::LazyLock;

use bacnet_types::enums::{EngineeringUnits, ObjectType};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use serde::Deserialize;
use serde_json::Value;

use super::session::AppState;
use crate::address::Address;
use crate::connection::BACNET_PORT;
use crate::points::{Point, StatusFlags};
use crate::present::unit_text;
use crate::scan::{Details, Device, ObjectList, RemoteStation};

#[derive(Deserialize)]
struct RememberedPoint {
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    value: String,
    #[serde(default)]
    units: Option<String>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    state_texts: Vec<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct RememberedPoints {
    points: Vec<RememberedPoint>,
}

#[derive(Deserialize)]
struct RememberedDevice {
    instance: u32,
    address: String,
    #[serde(default)]
    network: Option<u16>,
    #[serde(default)]
    mac: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    vendor: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    found_by: String,
}

/// Every unit by the text it is shown with, which is how the site file holds it.
static UNITS: LazyLock<HashMap<String, EngineeringUnits>> = LazyLock::new(|| {
    (0..=u32::from(u16::MAX))
        .map(EngineeringUnits::from_raw)
        .filter(|units| units.name().is_some())
        .map(|units| (unit_text(units), units))
        .collect()
});

fn point(remembered: RememberedPoint) -> Option<Point> {
    let object = ObjectIdentifier::new(
        ObjectType::from_raw(remembered.object_type),
        remembered.instance,
    )
    .ok()?;
    let units = remembered
        .units
        .as_deref()
        .and_then(|text| UNITS.get(text).copied());
    let plain = remembered
        .units
        .as_deref()
        .and_then(|text| remembered.value.strip_suffix(text))
        .map_or(remembered.value.as_str(), str::trim_end);
    let value = match plain.parse::<f32>() {
        _ if plain.is_empty() => None,
        Ok(number) if units.is_some() => Some(PropertyValue::Real(number)),
        _ => Some(PropertyValue::CharacterString(plain.to_string())),
    };
    let has = |word: &str| remembered.status.contains(word);
    let status = StatusFlags {
        in_alarm: has("alarm"),
        fault: has("fault"),
        overridden: has("overridden"),
        out_of_service: has("out of service"),
    };
    Some(Point {
        object,
        value,
        units,
        status: Some(status),
        name: remembered.name,
        description: remembered.description,
        state_texts: remembered.state_texts,
        error: remembered.error,
    })
}

fn device(remembered: RememberedDevice) -> Option<Device> {
    let address = remembered.address.parse().ok().or_else(|| {
        let ip = remembered.address.parse().ok()?;
        Some(Address::Ip(SocketAddrV4::new(ip, BACNET_PORT)))
    })?;
    let remote = remembered.network.map(|network| RemoteStation {
        network,
        mac: mac_bytes(remembered.mac.as_deref().unwrap_or_default()),
    });
    Some(Device {
        instance: remembered.instance,
        address,
        remote,
        answered_who_is: remembered.found_by == "who_is",
        details: Details {
            name: remembered.name,
            vendor: remembered.vendor,
            model: remembered.model,
            error: None,
        },
        objects: ObjectList::default(),
    })
}

fn mac_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

fn points_of(remembered: &Value) -> Option<Vec<Point>> {
    let list = RememberedPoints::deserialize(remembered).ok()?;
    Some(list.points.into_iter().filter_map(point).collect())
}

/// Fills in what the site remembers, keeping nothing else: a device read since stays as read.
pub(super) fn restore(state: &AppState) {
    let Some(site) = state.site.lock().expect("site lock").clone() else {
        return;
    };
    let devices: Vec<Device> = site
        .scan
        .as_ref()
        .and_then(|scan| scan["devices"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|found| RememberedDevice::deserialize(found).ok())
        .filter_map(device)
        .collect();
    *state.devices.lock().expect("devices lock") = devices;
    let mut readings = state.readings.lock().expect("readings lock");
    for (instance, remembered) in &site.points {
        if let (Ok(instance), Some(points)) = (instance.parse(), points_of(remembered)) {
            readings.entry(instance).or_insert(points);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::point_list_csv;
    use crate::present::shown_value;
    use serde_json::json;

    #[test]
    fn a_remembered_point_shows_and_exports_as_it_did() {
        let points = points_of(&json!({ "points": [
            { "type": 0, "instance": 3, "object": "Analog input 3", "name": "Supply", "value": "21.4 °C",
              "units": "°C", "status": "alarm, out of service", "state_texts": [] },
            { "type": 19, "instance": 1, "name": "Mode", "value": "Auto", "units": null, "status": "",
              "state_texts": ["Off", "Auto"] },
            { "type": 8, "instance": 7, "name": "Dead", "value": "", "error": "timeout" },
        ]}))
        .unwrap();
        assert_eq!(shown_value(&points[0]), "21.4 °C");
        assert_eq!(points[0].units, Some(EngineeringUnits::from_raw(62)));
        assert!(points[0].status.unwrap().out_of_service && !points[0].status.unwrap().fault);
        assert_eq!(shown_value(&points[1]), "Auto");
        assert_eq!(points[2].value, None);
        let csv = point_list_csv(5, &points);
        assert!(
            csv.contains("Supply;;21.4;°C;alarm, out of service;"),
            "{csv}"
        );
        assert!(csv.contains("Auto"), "{csv}");
    }

    #[test]
    fn a_remembered_device_keeps_its_address_and_place() {
        let plain = RememberedDevice::deserialize(&json!({
            "instance": 7, "address": "10.0.0.5", "found_by": "who_is", "name": "AHU",
        }))
        .unwrap();
        let plain = device(plain).unwrap();
        assert_eq!(plain.address, "10.0.0.5:47808".parse().unwrap());
        assert!(plain.answered_who_is);
        let routed = RememberedDevice::deserialize(&json!({
            "instance": 8, "address": "10.0.0.1:47809", "network": 5, "mac": "0A1B", "found_by": "direct",
        }))
        .unwrap();
        let routed = device(routed).unwrap();
        assert_eq!(routed.address.ip().unwrap().port(), 47809);
        assert_eq!(routed.remote.unwrap().mac, vec![0x0A, 0x1B]);
        assert!(!routed.answered_who_is);
    }

    #[test]
    fn something_broken_in_the_file_is_left_out() {
        assert!(points_of(&json!({ "points": "none" })).is_none());
        let bad = RememberedDevice::deserialize(&json!({ "instance": 1, "address": "nowhere" }));
        assert!(device(bad.unwrap()).is_none());
    }
}
