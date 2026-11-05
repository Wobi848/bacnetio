// The KBOB check of one device (docs/concept-kbob-check.md): what bacnetio read goes in, the
// findings per rule come out. No I/O, so the tests feed it devices that meet and break each rule.

import { ABBREVIATIONS, BIBBS, OBJECT_TYPES, PROPERTIES } from "./rules.js";

const DEVICE = 8;
const NOTIFICATION_CLASS = 15;
const ANALOG = [0, 1, 2];
const BINARY = [3, 4, 5];
const MULTISTATE = [13, 14, 19];
/** Object types from here on belong to a vendor (BACnet: 128 and above). */
const FIRST_PROPRIETARY = 128;
/** A read of all properties never lists Property_List itself (BACnet standard): it cannot be told. */
const NOT_IN_ALL = ["property-list"];

/** Positions in Protocol_Services_Supported (BACnetServicesSupported). */
const SERVICE_BITS = {
  "acknowledge-alarm": 0, "get-enrollment-summary": 4, "subscribe-cov": 5, "atomic-read-file": 6, "atomic-write-file": 7,
  "add-list-element": 8, "remove-list-element": 9, "create-object": 10, "delete-object": 11, "read-property": 12,
  "read-property-multiple": 14, "write-property": 15, "write-property-multiple": 16, "device-communication-control": 17,
  "reinitialize-device": 20, "who-has": 33, "who-is": 34, "read-range": 35, "utc-time-synchronization": 36,
  "subscribe-cov-property": 38, "get-event-information": 39,
};

/** Server BIBBs a device shows by the services it executes; the others cannot be read off a device. */
const BIBB_SERVICES = {
  "DS-RP-B": ["read-property"],
  "DS-RPM-B": ["read-property-multiple"],
  "DS-WP-B": ["write-property"],
  "DS-WPM-B": ["write-property-multiple"],
  "DS-COV-B": ["subscribe-cov"],
  "DS-COVP-B": ["subscribe-cov-property"],
  "AE-ACK-B": ["acknowledge-alarm"],
  "AE-ESUM-B": ["get-enrollment-summary"],
  "AE-INFO-B": ["get-event-information"],
  "T-VMT-I-B": ["read-range"],
  "DM-DDB-B": ["who-is"],
  "DM-DOB-B": ["who-has"],
  "DM-DCC-B": ["device-communication-control"],
  "DM-UTC-B": ["utc-time-synchronization"],
  "DM-RD-B": ["reinitialize-device"],
  "DM-BR-B": ["atomic-read-file", "atomic-write-file", "reinitialize-device"],
  "DM-LM-B": ["add-list-element", "remove-list-element"],
  "DM-OCD-B": ["create-object", "delete-object"],
};

export const RULES = ["revision", "objectTypes", "proprietary", "properties", "names", "units", "stateTexts", "notificationClass", "services"];
const SOURCES = { revision: "4.1", objectTypes: "5.2.1, C1.1", proprietary: "4.4", properties: "5.3, C1.2", names: "5.3.3", units: "5.3.4", stateTexts: "5.3.6", notificationClass: "5.3.5", services: "6, C1.3" };
export const sourceOf = (rule) => SOURCES[rule];

const byName = (properties) => new Map((properties ?? []).map((property) => [property.property, property]));
const has = (properties, name) => {
  const property = properties.get(name);
  return Boolean(property) && !/unknown-property/.test(property.error ?? "");
};
const text = (properties, name) => String(properties.get(name)?.value ?? "").trim();
const label = (object) => `${ABBREVIATIONS[object.type] ?? `type ${object.type}`} ${object.instance}`;

function revision(device) {
  const version = Number(text(device, "protocol-version"));
  const revisionNumber = Number(text(device, "protocol-revision"));
  if (!device.has("protocol-revision")) return { ok: null };
  return version === 1 && revisionNumber >= 14 ? { ok: true } : { ok: false, found: [`${version}.${revisionNumber}`] };
}

function objectTypes(device, profile) {
  const bits = device.get("protocol-object-types-supported")?.bits;
  if (!bits) return { ok: null };
  const missing = OBJECT_TYPES[profile].filter((type) => !bits[type]).map((type) => ABBREVIATIONS[type]);
  return { ok: !missing.length, found: missing };
}

function services(device, profile) {
  const bits = device.get("protocol-services-supported")?.bits;
  if (!bits) return { ok: null };
  const missing = BIBBS.filter((entry) => entry[profile] && BIBB_SERVICES[entry.bibb])
    .filter((entry) => BIBB_SERVICES[entry.bibb].some((service) => !bits[SERVICE_BITS[service]]))
    .map((entry) => entry.bibb);
  return { ok: !missing.length, found: missing };
}

/** One object's findings: each a rule and what is wrong, for its mark and the report. */
function objectFindings(object, profile, classes) {
  const findings = [];
  if (object.type >= FIRST_PROPRIETARY) findings.push({ rule: "proprietary", what: [String(object.type)] });
  if (!object.properties) return findings;
  const properties = byName(object.properties);
  const required = PROPERTIES[object.type]?.[profile] ?? [];
  const missing = required.map(([name]) => name).filter((name) => !NOT_IN_ALL.includes(name) && !has(properties, name));
  if (missing.length) findings.push({ rule: "properties", what: missing });
  if (!text(properties, "object-name")) findings.push({ rule: "names", what: ["—"] });
  if (ANALOG.includes(object.type) && (!text(properties, "units") || /^no.?units$/i.test(text(properties, "units")))) findings.push({ rule: "units", what: [] });
  if (BINARY.includes(object.type)) {
    const empty = ["active-text", "inactive-text"].filter((name) => !text(properties, name));
    if (empty.length) findings.push({ rule: "stateTexts", what: empty });
  }
  if (MULTISTATE.includes(object.type)) {
    const states = properties.get("state-text");
    const entries = states?.items ?? String(states?.value ?? "").split(",");
    if (!states || entries.some((entry) => !String(entry).trim())) findings.push({ rule: "stateTexts", what: ["state-text"] });
  }
  const reporting = (properties.get("event-enable")?.bits ?? []).some(Boolean);
  if (reporting) {
    const number = Number(text(properties, "notification-class"));
    if (!has(properties, "notification-class") || !classes.has(number)) findings.push({ rule: "notificationClass", what: [text(properties, "notification-class") || "—"] });
  }
  return findings;
}

/**
 * The check of one device. `device` is { instance, objects: [{ type, instance, name, properties }] },
 * the device object among them; `names` counts every object name across the devices checked, so a
 * name used twice anywhere is found (5.3.3).
 */
export function checkDevice(device, profile, names = new Map()) {
  const own = device.objects.find((object) => object.type === DEVICE);
  const deviceProperties = byName(own?.properties);
  const classes = new Set(device.objects.filter((object) => object.type === NOTIFICATION_CLASS).map((object) => object.instance));
  const objects = device.objects.map((object) => {
    const findings = objectFindings(object, profile, classes);
    const name = String(object.name ?? "").trim();
    if (name && (names.get(name) ?? 0) > 1) findings.push({ rule: "names", what: [name] });
    return { type: object.type, instance: object.instance, label: label(object), findings };
  });
  const perRule = (rule) => objects.filter((object) => object.findings.some((finding) => finding.rule === rule));
  const objectRule = (rule) => {
    const failing = perRule(rule);
    const unread = device.objects.some((object) => !object.properties);
    if (failing.length) return { ok: false, objects: failing.length };
    return unread && rule !== "proprietary" ? { ok: null } : { ok: true };
  };
  const rules = {
    revision: own ? revision(deviceProperties) : { ok: null },
    objectTypes: own ? objectTypes(deviceProperties, profile) : { ok: null },
    proprietary: objectRule("proprietary"),
    properties: objectRule("properties"),
    names: objectRule("names"),
    units: objectRule("units"),
    stateTexts: objectRule("stateTexts"),
    notificationClass: objectRule("notificationClass"),
    services: own ? services(deviceProperties, profile) : { ok: null },
  };
  // objects read in full and without findings, for their ✓ in bacnetio's lists
  const passed = device.objects.filter((object, index) => object.properties && !objects[index].findings.length)
    .map((object) => [object.type, object.instance]);
  return { profile, rules, objects: objects.filter((object) => object.findings.length), passed };
}

/** How often each object name occurs across the devices given. */
export function countNames(devices) {
  const names = new Map();
  for (const device of devices) {
    for (const object of device.objects) {
      const name = String(object.name ?? "").trim();
      if (name) names.set(name, (names.get(name) ?? 0) + 1);
    }
  }
  return names;
}
