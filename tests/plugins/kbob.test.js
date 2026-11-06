// The KBOB check without a browser: `node --test tests/plugins/*.test.js`.

import { test } from "node:test";
import assert from "node:assert/strict";

import { aksProblem, followsAks, splitName } from "../../plugins/kbob/aks.js";
import { RULES, checkDevice, countNames, defaultRules, kbobClass, withNewRules } from "../../plugins/kbob/check.js";
import { PROPERTIES } from "../../plugins/kbob/rules.js";
import { TEXTS } from "../../plugins/kbob/texts.js";

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
      object(0, 1, `Outdoor ${instance}`, { units: { value: "°C" }, "event-enable": { bits: [true, false, false] }, "notification-class": { value: "98" } }),
      object(3, 1, `Pump ${instance}`, { "active-text": { value: "On" }, "inactive-text": { value: "Off" } }),
      object(19, 1, `Mode ${instance}`, { "state-text": { value: "Off, Day, Night", items: ["Off", "Day", "Night"] } }),
      object(15, 98, `Alarms ${instance}`, { "ack-required": { bits: [true, true, false] }, priority: { value: "100, 100, 100", items: ["100", "100", "100"] } }),
    ],
  };
}

const check = (device, names = countNames([device])) => checkDevice(device, "AS-CH", names);
const failing = (result) => Object.entries(result.rules).filter(([, rule]) => rule.ok === false).map(([name]) => name);

test("a device that meets the profile has no findings", () => {
  const result = check(goodDevice());
  assert.deepEqual(failing(result), []);
  assert.deepEqual(result.objects, []);
  assert.equal(result.passed.length, 5, "every object read in full counts as met");
});

test("each rule finds what breaks it", () => {
  const cases = [
    ["revision", (device) => { device.objects[0].properties.find((p) => p.property === "protocol-revision").value = "12"; }],
    ["objectTypes", (device) => { device.objects[0].properties.find((p) => p.property === "protocol-object-types-supported").bits[17] = false; }],
    ["services", (device) => { device.objects[0].properties.find((p) => p.property === "protocol-services-supported").bits[14] = false; }],
    ["proprietary", (device) => { device.objects.push({ type: 130, instance: 1, name: "Vendor thing", properties: [prop("object-name", "Vendor thing")] }); }],
    ["properties", (device) => { device.objects[1].properties = device.objects[1].properties.filter((p) => p.property !== "device-type"); }],
    ["units", (device) => { device.objects[1].properties.find((p) => p.property === "units").value = ""; }],
    ["stateTexts", (device) => { device.objects[2].properties.find((p) => p.property === "active-text").value = ""; }],
    ["notificationClass", (device) => { device.objects[1].properties.find((p) => p.property === "notification-class").value = "9"; }],
    ["kbobClasses", (device) => { device.objects[4].properties.find((p) => p.property === "priority").items = ["100", "120", "100"]; }],
    ["kbobClasses", (device) => { device.objects[4].properties.find((p) => p.property === "ack-required").bits = [false, false, false]; }],
  ];
  for (const [rule, breakIt] of cases) {
    const device = goodDevice();
    breakIt(device);
    assert.deepEqual(failing(check(device)), [rule], rule);
  }
});

const slots = (taken) => Array.from({ length: 16 }, (_, index) => taken[index + 1] ?? "null");

test("table 10: commands at its priorities pass, a free priority or a schedule off 14 is found", () => {
  const withValve = (taken) => {
    const device = goodDevice();
    device.objects.push(object(1, 1, "Valve 1", { units: { value: "%" }, "priority-array": { value: "", items: slots(taken) } }));
    return device;
  };
  assert.deepEqual(failing(check(withValve({ 8: "40 %", 16: "20 %" }))), [], "manual from the MBE and automatic");
  const free = check(withValve({ 9: "40 %", 13: "10 %" }));
  assert.deepEqual(failing(free), ["priorities"]);
  assert.deepEqual(free.objects.find((entry) => entry.label === "AO 1").findings, [{ rule: "priorities", what: ["9", "13"] }]);

  const withSchedule = (priority) => {
    const device = goodDevice();
    device.objects.push(object(17, 1, "Time program 1", { "priority-for-writing": { value: priority } }));
    return device;
  };
  assert.deepEqual(failing(check(withSchedule("14"))), []);
  const off = check(withSchedule("16"));
  assert.deepEqual(off.objects.find((entry) => entry.label === "SCHED 1").findings, [{ rule: "priorities", what: ["16"], schedule: true }]);
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

test("KBOB notification classes after tables 7 and 8", () => {
  assert.equal(kbobClass(5), null);
  assert.deepEqual(kbobClass(1), { acknowledged: [false, false, false], lowest: 1, highest: 7 });
  assert.deepEqual(kbobClass(98), { acknowledged: [true, true, false], lowest: 96, highest: 103 });
  assert.deepEqual(kbobClass(225), { acknowledged: [false, true, false], lowest: 224, highest: 231 });
  assert.equal(kbobClass(226), null);
});

test("object names after the AKS pattern of the KBOB guide, only when asked for", () => {
  assert.ok(followsAks(0, "3022_HZ1_L13_M100_M0"));
  // the FUNKTION fits the object type (recommendation 7.2, guide table 2)
  assert.deepEqual(aksProblem(0, "3022_HZ1_L13_M100_S0"), { letter: "S", expected: "M/Z" }, "a switching command is no analog input");
  assert.equal(aksProblem(2, "3022_HZ1_L13_P100_Z1"), null, "a meter value is an AV");
  assert.equal(aksProblem(4, "3022_HZ1_L13_M100_S0"), null);
  assert.deepEqual(aksProblem(3, "3022_HZ1_L13_M100_S0"), { letter: "S", expected: "D" });
  assert.ok(followsAks(8, "3022_HZ1_L13_D001"), "a device ends with its APPARAT");
  assert.ok(!followsAks(0, "3022_HZ1_L13_M100"), "a point needs its FUNKTION");
  assert.ok(!followsAks(0, "Outdoor air temp"));
  assert.ok(!followsAks(0, "3022_HZ1_I13_M100_M0"), "I is no BTA kind");
  assert.ok(!followsAks(15, "Alarms"), "notification classes are named too (guide 3.2.2, N)");
  assert.ok(followsAks(15, "3022_HZ1_L13_I001_N1"));
  assert.equal(aksProblem(0, "3022_HZ1_L13_P100_Z1"), null, "a meter value may be an AI");
  assert.equal(aksProblem(9, "3022_HZ1_L13_I100_H1"), null, "Event Enrollment has no letter of its own");
  assert.deepEqual(splitName("3022_HZ1_L13_M100_S0").parts, { unit: "3022", object: "HZ1", system: "L13", device: "M100", function: "S0" });

  const device = goodDevice();
  assert.equal(check(device).rules.aks, undefined, "off unless asked");
  const result = checkDevice(device, "AS-CH", countNames([device]), { rules: ["names", "aks"] });
  assert.equal(result.rules.aks.ok, false);
});

test("only the rules chosen are checked and marked", () => {
  const device = goodDevice();
  device.objects[1].properties.find((p) => p.property === "units").value = "";
  assert.deepEqual(failing(check(device)), ["units"]);
  const without = checkDevice(device, "AS-CH", countNames([device]), { rules: ["revision", "properties"] });
  assert.deepEqual(Object.keys(without.rules), ["revision", "properties"]);
  assert.deepEqual(without.objects, [], "no finding of a rule left out");
  assert.equal(without.passed.length, 5);
});

test("only what the recommendation asks counts: no-units is a unit, outputs need no state texts", () => {
  const device = goodDevice();
  device.objects[1].properties.find((p) => p.property === "units").value = "no-units";
  device.objects.push(object(4, 1, "Fan 1", { "active-text": { value: "" }, "inactive-text": { value: "" } }));
  assert.deepEqual(failing(check(device)), []);
});

test("every language of the plugin has every text, with the same placeholders", () => {
  const entries = (texts, prefix = "") => Object.entries(texts).flatMap(([key, value]) =>
    (typeof value === "object" ? entries(value, `${prefix}${key}.`) : [[`${prefix}${key}`, value]]));
  const english = new Map(entries(TEXTS.en));
  assert.deepEqual(Object.keys(TEXTS), ["en", "de", "fr", "it", "es"]);
  for (const [language, texts] of Object.entries(TEXTS)) {
    const own = new Map(entries(texts));
    assert.deepEqual([...own.keys()].sort(), [...english.keys()].sort(), language);
    for (const [key, text] of own) {
      const holes = (words) => (words.match(/\{\w+\}/g) ?? []).sort().join();
      assert.equal(holes(text), holes(english.get(key)), `${language} ${key}`);
    }
  }
});

test("a rule that came with an update is on, one the user switched off stays off", () => {
  // as if kbobClasses came with an update, and the user had switched off units
  const before = RULES.filter((rule) => rule !== "kbobClasses");
  const chosen = before.filter((rule) => rule !== "units" && rule !== "aks");
  const now = withNewRules({ rules: chosen, offered: before });
  assert.ok(now.includes("kbobClasses"), "the new rule is on");
  assert.ok(!now.includes("units"), "switched off by the user");
  assert.ok(!now.includes("aks"), "off by default");
  assert.deepEqual(withNewRules({ rules: RULES, offered: RULES }), RULES, "AKS chosen stays");
  assert.deepEqual(withNewRules({ rules: defaultRules(), offered: RULES }), defaultRules());
  // kept before rules were offered: every default rule not chosen counts as new
  assert.deepEqual(withNewRules({ rules: ["revision"] }), defaultRules());
});
