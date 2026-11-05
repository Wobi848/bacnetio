// The KBOB check without a browser: `node --test tests/plugins/*.test.js`.

import { test } from "node:test";
import assert from "node:assert/strict";

import { checkDevice, countNames } from "../../plugins/kbob/check.js";
import { PROPERTIES } from "../../plugins/kbob/rules.js";

const ALL_BITS = (length) => Array.from({ length }, () => true);
const prop = (property, value = "x", extra = {}) => ({ property, value, error: null, ...extra });

/** Every property the profile asks of the type, with a value; `change` adjusts single ones. */
function object(type, instance, name, change = {}) {
  const properties = (PROPERTIES[type]?.["AS-CH"] ?? []).map(([property]) => prop(property));
  for (const [property, value] of Object.entries(change)) {
    const index = properties.findIndex((entry) => entry.property === property);
    if (value === undefined) properties.splice(index, 1);
    else if (index >= 0) properties[index] = { ...properties[index], ...value };
    else properties.push(prop(property, value.value, value));
  }
  return { type, instance, name, properties };
}

function goodDevice(instance = 1) {
  return {
    instance,
    objects: [
      object(8, instance, `Device ${instance}`, {
        "protocol-version": { value: "1" },
        "protocol-revision": { value: "14" },
        "protocol-object-types-supported": { bits: ALL_BITS(64) },
        "protocol-services-supported": { bits: ALL_BITS(49) },
      }),
      object(0, 1, `Outdoor ${instance}`, { units: { value: "°C" }, "event-enable": { bits: [true, false, false] }, "notification-class": { value: "5" } }),
      object(3, 1, `Pump ${instance}`, { "active-text": { value: "On" }, "inactive-text": { value: "Off" } }),
      object(19, 1, `Mode ${instance}`, { "state-text": { value: "Off, Day, Night", items: ["Off", "Day", "Night"] } }),
      object(15, 5, `Alarms ${instance}`),
    ],
  };
}

const check = (device, names = countNames([device])) => checkDevice(device, "AS-CH", names);
const failing = (result) => Object.entries(result.rules).filter(([, rule]) => rule.ok === false).map(([name]) => name);

test("a device that meets the profile has no findings", () => {
  const result = check(goodDevice());
  assert.deepEqual(failing(result), []);
  assert.deepEqual(result.objects, []);
});

test("each rule finds what breaks it", () => {
  const cases = [
    ["revision", (device) => { device.objects[0].properties.find((p) => p.property === "protocol-revision").value = "12"; }],
    ["objectTypes", (device) => { device.objects[0].properties.find((p) => p.property === "protocol-object-types-supported").bits[17] = false; }],
    ["services", (device) => { device.objects[0].properties.find((p) => p.property === "protocol-services-supported").bits[14] = false; }],
    ["proprietary", (device) => { device.objects.push({ type: 130, instance: 1, name: "Vendor thing", properties: [prop("object-name", "Vendor thing")] }); }],
    ["properties", (device) => { device.objects[1].properties = device.objects[1].properties.filter((p) => p.property !== "device-type"); }],
    ["units", (device) => { device.objects[1].properties.find((p) => p.property === "units").value = "no-units"; }],
    ["stateTexts", (device) => { device.objects[2].properties.find((p) => p.property === "active-text").value = ""; }],
    ["notificationClass", (device) => { device.objects[1].properties.find((p) => p.property === "notification-class").value = "9"; }],
  ];
  for (const [rule, breakIt] of cases) {
    const device = goodDevice();
    breakIt(device);
    assert.deepEqual(failing(check(device)), [rule], rule);
  }
});

test("a property the device does not have counts as missing, Property_List is never asked for", () => {
  const device = goodDevice();
  device.objects[1].properties.find((p) => p.property === "resolution").error = "unknown-property";
  const result = check(device);
  const outdoor = result.objects.find((entry) => entry.label === "AI 1");
  assert.deepEqual(outdoor.findings, [{ rule: "properties", what: ["resolution"] }]);
});

test("an object name used twice anywhere in the site is found on both", () => {
  const first = goodDevice(1);
  const second = goodDevice(2);
  second.objects[1].name = "Outdoor 1";
  const names = countNames([first, second]);
  assert.deepEqual(failing(checkDevice(first, "AS-CH", names)), ["names"]);
  assert.deepEqual(failing(checkDevice(second, "AS-CH", names)), ["names"]);
});

test("objects not read leave their rules open instead of passing them", () => {
  const device = goodDevice();
  device.objects[1].properties = null;
  const result = check(device);
  assert.equal(result.rules.properties.ok, null);
  assert.equal(result.rules.revision.ok, true);
});
