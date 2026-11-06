// The KBOB check of one device (docs/concept-kbob-check.md): what bacnetio read goes in, the
// findings per rule come out. No I/O, so the tests feed it devices that meet and break each rule.

import { aksProblem } from "./aks.js";
import { ABBREVIATIONS, BIBBS, OBJECT_TYPES, PROPERTIES } from "./rules.js";

const DEVICE = 8;
const NOTIFICATION_CLASS = 15;
const SCHEDULE = 17;
/** Table 10: priorities KBOB gives no use, free only after agreement with the owner. */
const FREE_PRIORITIES = [3, 4, 9, 10, 11, 13, 15];
/** Table 10: the BIBB Scheduling (central time programs) writes at priority 14. */
const SCHEDULE_PRIORITY = 14;
const ANALOG = [0, 1, 2];
/** 5.3.6 names binary and multi-state inputs and values for state texts, not outputs. */
const BINARY = [3, 5];
const MULTISTATE = [13, 19];
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

/**
 * The KBOB notification classes (recommendation, table 7): eight categories from NC1, NC32 … NC224,
 * four classes each, two for trend data and status. Table 8: within a category, which transitions
 * need acknowledging [TO-OFFNORMAL, TO-FAULT, TO-NORMAL], and the range of priorities.
 */
const CATEGORIES = [[1, 4, 1, 7], [32, 4, 32, 39], [64, 4, 64, 71], [96, 4, 96, 103], [128, 4, 128, 135], [160, 4, 160, 167], [192, 2, 192, 199], [224, 2, 224, 231]];
const ACKNOWLEDGED = [[false, false, false], [false, true, false], [true, true, false], [true, true, true]];

/** Table 8 for a class number, or null when it is no KBOB class. */
export function kbobClass(number) {
  const category = CATEGORIES.find(([first, count]) => number >= first && number < first + count);
  if (!category) return null;
  const [first, , lowest, highest] = category;
  return { acknowledged: ACKNOWLEDGED[number - first], lowest, highest };
}

/** Rules a check leaves out unless the user picks them: AKS is an example of the guide, no rule. */
export const OFF_BY_DEFAULT = ["aks"];
export const defaultRules = () => RULES.filter((rule) => !OFF_BY_DEFAULT.includes(rule));

export const RULES = ["revision", "objectTypes", "proprietary", "properties", "names", "aks", "units", "stateTexts", "notificationClass", "kbobClasses", "priorities", "services"];

/**
 * The rules chosen, from a kept choice: rules that came with an update since it was saved are on,
 * unless off by default; what the user switched off stays off. A choice kept before `offered` was
 * saved counts every rule but the chosen and the off ones as new.
 */
export function withNewRules({ rules, offered = [...rules, ...OFF_BY_DEFAULT] }) {
  return RULES.filter((rule) => rules.includes(rule) || (!offered.includes(rule) && !OFF_BY_DEFAULT.includes(rule)));
}
const SOURCES = { revision: "4.1", objectTypes: "5.2.1, C1.1", proprietary: "4.4", properties: "5.3, C1.2", names: "5.3.3", aks: "Leitfaden 3.2.2", units: "5.3.4", stateTexts: "5.3.6", notificationClass: "5.3.5", kbobClasses: "6.2, Tab. 7, 8", priorities: "6.3.2, Tab. 10", services: "6, C1.3" };
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

/** A notification class object against tables 7 and 8: its number, acknowledgement and priorities. */
function classFindings(object, properties) {
  const kbob = kbobClass(object.instance);
  if (!kbob) return [`NC${object.instance}`];
  const what = [];
  const acknowledged = properties.get("ack-required")?.bits;
  if (acknowledged && kbob.acknowledged.some((wanted, index) => Boolean(acknowledged[index]) !== wanted)) what.push("ack-required");
  const priority = properties.get("priority");
  const priorities = (priority?.items ?? String(priority?.value ?? "").split(",")).map((entry) => Number(String(entry).trim()));
  if (priority && priorities.some((value) => !(value >= kbob.lowest && value <= kbob.highest))) what.push("priority");
  return what;
}

/** Commands held at priorities table 10 leaves free, and a schedule writing elsewhere than 14. */
function priorityFindings(object, properties) {
  const slots = properties.get("priority-array")?.items ?? [];
  const taken = slots.map((value, index) => [index + 1, String(value).trim()])
    .filter(([priority, value]) => FREE_PRIORITIES.includes(priority) && value && value !== "null")
    .map(([priority]) => String(priority));
  const findings = taken.length ? [{ rule: "priorities", what: taken }] : [];
  const writing = text(properties, "priority-for-writing");
  if (object.type === SCHEDULE && writing && Number(writing) !== SCHEDULE_PRIORITY) {
    findings.push({ rule: "priorities", what: [writing], schedule: true });
  }
  return findings;
}

/** One object's findings: each a rule and what is wrong, for its mark and the report. */
function objectFindings(object, profile, classes, options) {
  const findings = [];
  if (object.type >= FIRST_PROPRIETARY) findings.push({ rule: "proprietary", what: [String(object.type)] });
  if (!object.properties) return findings;
  const properties = byName(object.properties);
  const required = PROPERTIES[object.type]?.[profile] ?? [];
  const missing = required.map(([name]) => name).filter((name) => !NOT_IN_ALL.includes(name) && !has(properties, name));
  if (missing.length) findings.push({ rule: "properties", what: missing });
  if (!text(properties, "object-name")) findings.push({ rule: "names", what: ["—"] });
  else if (options.rules.includes("aks")) {
    const name = text(properties, "object-name");
    const problem = aksProblem(object.type, name);
    if (problem?.pattern) findings.push({ rule: "aks", what: [name] });
    else if (problem) findings.push({ rule: "aks", what: [name], letter: problem.letter, expected: problem.expected });
  }
  if (object.type === NOTIFICATION_CLASS) {
    const wrong = classFindings(object, properties);
    if (wrong.length) findings.push({ rule: "kbobClasses", what: wrong });
  }
  findings.push(...priorityFindings(object, properties));
  // 5.3.4 asks for a BACnetEngineeringUnit; no-units is one of them, so only a missing unit counts
  if (ANALOG.includes(object.type) && !text(properties, "units")) findings.push({ rule: "units", what: [] });
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
 * The check of one device, by the rules in `options.rules` (all but AKS unless given). `device` is { instance, objects: [{ type, instance, name, properties }] },
 * the device object among them; `names` counts every object name across the devices checked, so a
 * name used twice anywhere is found (5.3.3).
 */
export function checkDevice(device, profile, names = new Map(), given = {}) {
  const options = { rules: defaultRules(), ...given };
  const own = device.objects.find((object) => object.type === DEVICE);
  const deviceProperties = byName(own?.properties);
  const classes = new Set(device.objects.filter((object) => object.type === NOTIFICATION_CLASS).map((object) => object.instance));
  const objects = device.objects.map((object) => {
    const findings = objectFindings(object, profile, classes, options);
    const name = String(object.name ?? "").trim();
    if (name && (names.get(name) ?? 0) > 1) findings.push({ rule: "names", what: [name] });
    const chosen = findings.filter((finding) => options.rules.includes(finding.rule));
    return { type: object.type, instance: object.instance, label: label(object), findings: chosen };
  });
  const perRule = (rule) => objects.filter((object) => object.findings.some((finding) => finding.rule === rule));
  const objectRule = (rule) => {
    const failing = perRule(rule);
    const unread = device.objects.some((object) => !object.properties);
    if (failing.length) return { ok: false, objects: failing.length };
    return unread && rule !== "proprietary" ? { ok: null } : { ok: true };
  };
  const all = {
    revision: own ? revision(deviceProperties) : { ok: null },
    objectTypes: own ? objectTypes(deviceProperties, profile) : { ok: null },
    proprietary: objectRule("proprietary"),
    properties: objectRule("properties"),
    names: objectRule("names"),
    aks: objectRule("aks"),
    units: objectRule("units"),
    stateTexts: objectRule("stateTexts"),
    notificationClass: objectRule("notificationClass"),
    kbobClasses: objectRule("kbobClasses"),
    priorities: objectRule("priorities"),
    services: own ? services(deviceProperties, profile) : { ok: null },
  };
  const rules = Object.fromEntries(Object.entries(all).filter(([rule]) => options.rules.includes(rule)));
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
