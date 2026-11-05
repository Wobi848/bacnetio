// UI logic without a browser: `node --test tests/web/*.test.js`. A stub stands in for the document.

import { test } from "node:test";
import assert from "node:assert/strict";

globalThis.document = { documentElement: {} };
const { LANGUAGE_FILES, LANGUAGES, errorText, noteText, objectText, propertyValueText, setLanguage, t, valueWord } = await import("../../web/i18n.js");
const chartModule = await import("../../web/chart.js");
const { axisTexts, linePath, scale } = chartModule;
const { viewRecording } = await import("../../web/recording.js");
const { viewDevices } = await import("../../web/devices.js");
const { chooseAdapter, deviceMaker, stateWord } = await import("../../web/format.js");
const { groupPoints } = await import("../../web/grouping.js");
const { viewPoints } = await import("../../web/points.js");
const { nextSort, sortRows } = await import("../../web/sorting.js");
const { byUnit, figures, inWindow, thinned } = await import("../../web/analysis.js");
const { OUTSIDE, nodePoints, outsidePoints, plantNodes } = await import("../../web/structure.js");
const { bbmdHtml } = await import("../../web/bbmd.js");

const baseState = () => ({
  readAt: {},
  recView: "cards", recWindow: null, recZoom: null, recColors: {}, allRows: false, pointTypes: [], pointSort: null, deviceSort: null, winners: {}, node: null, watch: [], sites: [], sitePoints: {}, treeOpen: {}, structure: {},
  area: "devices", held: [], recording: null, scan: { state: "idle" }, objectNames: {}, menu: null,
  settings: { bbmd: "", port: "47808", locate: "", idFrom: "", idTo: "", subnet: "" }, adapter: { name: "Ethernet", ip: "10.0.0.2", prefix_len: 24 },
  bbmd: {}, bbmdReading: null,
  deviceFilter: "", selectedDevice: null, deviceFacts: null, filter: "", pointView: "all", groupsOpen: {}, reference: null,
  selectedPoint: null, object: null, pointNotes: [], error: null, writeUnlocked: false, asking: false,
});
const point = (type, instance, name, extra = {}) => ({ type, instance, name, description: "", value: "1", units: null, status: "", state_texts: [], error: null, ...extra });

test("texts fill placeholders and follow the language", () => {
  setLanguage("en");
  assert.equal(t("found", { n: 3 }), "3 found");
  setLanguage("de");
  assert.equal(t("found", { n: 3 }), "3 gefunden");
  assert.equal(t("no.such.key"), "no.such.key");
  setLanguage("en");
});

test("values in German use words and decimal commas", () => {
  setLanguage("de");
  assert.equal(valueWord("active"), "aktiv");
  assert.equal(valueWord("state 2"), "Zustand 2");
  assert.equal(valueWord("21.5 °C"), "21,5 °C");
  assert.equal(valueWord("192.168.1.10"), "192.168.1.10");
  assert.equal(objectText({ type: 1, instance: 4, object: "Analog output 4" }), "Analogausgang 4");
  setLanguage("en");
  assert.equal(valueWord("21.5 °C"), "21.5 °C");
  assert.equal(objectText({ type: 999, instance: 1, object: "Type 999 1" }), "Type 999 1");
});

test("server notes are translated by code, English comes from the server", () => {
  const note = { code: "duplicate_id", instance: 7, addresses: ["10.0.0.1", "10.0.0.2"], text: "Device ID 7 answers from ..." };
  setLanguage("en");
  assert.equal(noteText(note), note.text);
  setLanguage("de");
  assert.match(noteText(note), /Geräte-ID 7 antwortet von 10\.0\.0\.1, 10\.0\.0\.2/);
  assert.equal(noteText({ code: "unknown_code", text: "fallback" }), "fallback");
  setLanguage("en");
});

test("the chart line breaks at gaps and steps for binary values", () => {
  const samples = [[0, 1], [1000, null], [2000, 0], [3000, 1]];
  const s = scale(samples);
  const line = linePath(samples, s, false);
  assert.equal(line.match(/M/g).length, 2);
  const stepped = linePath(samples, s, true);
  assert.match(stepped, /H[\d.]+V[\d.]+/);
});

test("a flat series still gets a scale", () => {
  const s = scale([[0, 5], [1000, 5]]);
  assert.ok(s.high > s.low);
});

test("the recording page explains what to do when empty", () => {
  const view = viewRecording({ ...baseState(), area: "recording", recording: { running: false, points: [] } });
  assert.match(view.html, /Nothing recorded yet/);
  assert.equal(view.next, null);
});

test("device names from the network are escaped", () => {
  const state = baseState();
  state.scan = {
    state: "done",
    devices: [{ instance: 1, address: "10.0.0.5", network: null, mac: null, name: "<img src=x onerror=alert(1)>", vendor: "V", model: null, found_by: "who_is", error: null }],
    notes: [],
    routers: [],
  };
  const view = viewDevices(state);
  assert.doesNotMatch(view.html, /<img/);
  assert.match(view.html, /&lt;img/);
});

test("known server errors are translated, unknown ones stay as they are", () => {
  setLanguage("de");
  assert.equal(errorText("search for devices first"), "Zuerst nach Geräten suchen.");
  assert.equal(errorText("the device refused: write access denied"), "Das Gerät hat abgelehnt: write access denied");
  assert.equal(errorText("\"warm\" is not a number"), "„warm“ ist keine Zahl.");
  assert.equal(errorText("something new"), "something new");
  setLanguage("en");
  assert.equal(errorText("search for devices first"), "search for devices first");
});

test("a property's value keeps the device's words, the cut of a long list is translated", () => {
  const supported = "subscribe-cov, read-property, read-range";
  setLanguage("de");
  assert.equal(propertyValueText(supported), supported);
  assert.equal(propertyValueText(`${supported}, … 5 more`), `${supported}, … 5 weitere`);
  setLanguage("en");
  assert.equal(propertyValueText(`${supported}, … 5 more`), `${supported}, … 5 more`);
});

test("vendor and model without the brand twice, initials without company forms", () => {
  const device = { instance: 21, name: "DDC4000_#21", vendor: "Kieback&Peter GmbH&Co KG", model: "Kieback&Peter DDC4000", address: "192.168.10.21", network: null };
  assert.equal(deviceMaker(device), "Kieback&Peter DDC4000");
  assert.equal(deviceMaker({ vendor: "Siemens", model: "PXC4" }), "Siemens PXC4");
  assert.equal(deviceMaker({ vendor: null, model: null }), "");
  const html = viewDevices({ ...baseState(), scan: { state: "done", devices: [device], notes: [], routers: [], refused: [] } }).html;
  assert.match(html, /Kieback&amp;Peter DDC4000/);
  assert.doesNotMatch(html, /GmbH/);
});

test("points named as paths are grouped by plant part, without the shared start", () => {
  const points = [
    point(13, 1, "/21/000/00/00/SY_Datsi.01/state"),
    point(0, 2, "/21/000/00/00/SY_Datsi.01/procBar"),
    point(1, 3, "/21/000/09/11/S337.01/5101"),
    point(2, 4, "Outside"),
  ];
  const groups = groupPoints(points);
  assert.deepEqual(groups.map((group) => group.title), ["00/00/SY_Datsi.01", "09/11/S337.01", "other"]);
  assert.deepEqual(groups[0].items.map((item) => item.name), ["state", "procBar"]);
  assert.equal(groups[2].kind, true);
});

test("points without paths are grouped by kind", () => {
  const groups = groupPoints([point(0, 1, "Supply air"), point(1, 1, "Valve"), point(19, 1, "Mode")]);
  assert.deepEqual(groups.map((group) => group.title), ["inputs", "outputs", "values"]);
  assert.equal(groups[0].items[0].name, "Supply air");
});

/** Structured views as the API sends them: a root that holds a view and an object elsewhere. */
const views = () => [
  {
    type: 29, instance: 1, object: "Structured view 1", name: "Plant", error: null,
    subordinates: [
      { type: 29, instance: 2, object: "Structured view 2", device: null, annotation: "Air handling", name: "Air handling" },
      { type: 0, instance: 7, object: "Analog input 7", device: 1002, annotation: "Weather station", name: null },
    ],
  },
  {
    type: 29, instance: 2, object: "Structured view 2", name: "Air handling", error: null,
    subordinates: [
      { type: 0, instance: 2, object: "Analog input 2", device: null, annotation: "Outdoor air", name: "Outdoor air temp" },
      { type: 0, instance: 1, object: "Analog input 1", device: null, annotation: "Supply air", name: "Supply air temp" },
    ],
  },
];

test("a device's views become a plant tree, nested views indented below their root", () => {
  const nodes = plantNodes(views());
  assert.deepEqual(nodes.map((node) => [node.key, node.title, node.depth]), [
    ["view:1", "Plant", 0],
    ["view:2", "Air handling", 1],
  ]);
  // the root counts what hangs below it, through the nested view
  assert.deepEqual(nodes[0].objects, ["0:2", "0:1"]);
  assert.equal(nodes[0].elsewhere, 1, "the object on device 1002 is counted, not listed");
  assert.deepEqual(nodes[1].objects, ["0:2", "0:1"]);
});

test("a view with no name is called what the view above it calls it", () => {
  const [root, air] = views();
  const nodes = plantNodes([root, { ...air, name: null }]);
  assert.equal(nodes[1].title, "Air handling");
  assert.equal(plantNodes([{ ...root, name: null, subordinates: [] }])[0].title, "Structured view 1");
});

test("views that hold each other each start a branch and are walked once", () => {
  const circle = [
    { type: 29, instance: 1, object: "Structured view 1", name: "A", subordinates: [{ type: 29, instance: 2, device: null }] },
    { type: 29, instance: 2, object: "Structured view 2", name: "B", subordinates: [{ type: 29, instance: 1, device: null }] },
  ];
  const nodes = plantNodes(circle);
  assert.deepEqual(nodes.map((node) => node.title), ["A", "B", "B", "A"]);
});

test("the points no view holds stay reachable, the views themselves are no rows", () => {
  const points = [point(0, 1, "Supply air temp"), point(0, 2, "Outdoor air temp"), point(17, 1, "Office hours"), point(29, 1, "Plant"), point(29, 2, "Air handling")];
  const nodes = plantNodes(views());
  assert.deepEqual(outsidePoints(nodes, points).map((item) => item.name), ["Office hours"]);
  // a node shows what its views name, in their order
  assert.deepEqual(nodePoints(nodes, "view:2", points).map((item) => item.name), ["Outdoor air temp", "Supply air temp"]);
  assert.deepEqual(nodePoints(nodes, OUTSIDE, points).map((item) => item.name), ["Office hours"]);
  assert.deepEqual(nodePoints(nodes, "view:99", points), []);
});

test("the table of a plant node shows that node's points only", () => {
  const state = baseState();
  state.area = "points";
  state.device = { instance: 1001, name: "AHU-01", vendor: "V", model: "M", address: "10.0.0.1", network: null };
  state.points = [point(0, 1, "Supply air temp"), point(0, 2, "Outdoor air temp"), point(17, 1, "Office hours")];
  state.structure = { 1001: views() };
  state.node = "view:2";
  const html = viewPoints(state).html;
  assert.match(html, /Supply air temp/);
  assert.doesNotMatch(html, /Office hours/);
  state.node = OUTSIDE;
  const outside = viewPoints(state).html;
  assert.match(outside, /Office hours/);
  assert.doesNotMatch(outside, /Supply air temp/);
});

test("multi-state values use the device's state texts", () => {
  setLanguage("de");
  assert.equal(stateWord("state 2", ["Aus", "Betrieb"]), "Betrieb");
  assert.equal(stateWord("state 3", ["Aus", "Betrieb"]), "Zustand 3");
  setLanguage("en");
});

test("the point list filters faults and shows only matching groups", () => {
  const state = baseState();
  state.area = "points";
  state.device = { instance: 21, name: "DDC", vendor: "K", model: "M", address: "10.0.0.21", network: null };
  state.points = [point(0, 1, "/a/b/Pump.01/temp"), point(0, 2, "/a/b/Boiler.01/temp", { status: "fault" })];
  state.pointView = "faults";
  const html = viewPoints(state).html;
  assert.match(html, /Boiler\.01/);
  assert.doesNotMatch(html, /Pump\.01/);
});

test("the BBMD tables offer to be read, then show both tables in words", () => {
  const state = { ...baseState(), selectedDevice: 300 };
  const before = bbmdHtml(state);
  assert.match(before, /data-act="bbmd-read"/);
  assert.match(before, /Read BBMD tables/);
  state.bbmd = {
    300: {
      is_bbmd: true,
      bdt: [
        { address: "10.0.0.1", mask: "255.255.255.255", own: true },
        { address: "10.1.0.1", mask: "255.255.255.0", own: false },
      ],
      fdt: [{ address: "10.2.0.9:47809", ttl_s: 300, seconds_left: 262 }],
      bdt_note: null,
      fdt_note: null,
    },
  };
  const html = bbmdHtml(state);
  assert.match(html, /This device is a BBMD/);
  assert.match(html, /10\.0\.0\.1<\/dt><dd>to the BBMD itself · this BBMD/);
  assert.match(html, /to its subnet \(255\.255\.255\.0\)/);
  assert.match(html, /300 s asked for · 262 s left/);
  // a device that is no BBMD says so in words, by the note's code
  const refusal = "The device refused the table read: it is no BBMD, or it does not give its tables.";
  state.bbmd = { 300: { is_bbmd: false, bdt: null, fdt: null, bdt_note: { code: "no_bbmd", text: refusal }, fdt_note: { code: "silent", text: "No answer." } } };
  const refused = bbmdHtml(state);
  assert.doesNotMatch(refused, /This device is a BBMD/);
  assert.match(refused, /it is no BBMD/);
  setLanguage("de");
  assert.match(bbmdHtml(state), /kein BBMD/);
  setLanguage("en");
  state.bbmdReading = 300;
  assert.match(bbmdHtml(state), /Reading the BBMD tables/);
});

test("every language has every text, note, error and object type", () => {
  const sameKeys = (part, reference) => {
    const expected = Object.keys(reference).sort();
    for (const [language, file] of Object.entries(LANGUAGE_FILES)) {
      if (language === "en" && part !== "texts" && part !== "objectTypes") continue;
      const keys = Object.keys(file[part]).sort();
      assert.deepEqual(expected.filter((key) => !keys.includes(key)), [], `${part} missing in ${language}`);
      assert.deepEqual(keys.filter((key) => !expected.includes(key)), [], `${part} only in ${language}`);
    }
  };
  // English notes and errors come from the server; German has them all
  sameKeys("texts", LANGUAGE_FILES.en.texts);
  sameKeys("objectTypes", LANGUAGE_FILES.en.objectTypes);
  sameKeys("notes", LANGUAGE_FILES.de.notes);
  sameKeys("errors", LANGUAGE_FILES.de.errors);
});

test("only reviewed languages are in the menu", () => {
  assert.deepEqual(LANGUAGES.map(([code]) => code), ["en", "de"]);
  setLanguage("fr");
  assert.equal(t("tab.devices"), "Appareils");
  assert.equal(valueWord("21.5 °C"), "21,5 °C");
  setLanguage("en");
});

test("every text key used in the interface exists", async () => {
  const { readdir, readFile } = await import("node:fs/promises");
  const folder = new URL("../../web/", import.meta.url);
  const used = new Set();
  for (const file of (await readdir(folder)).filter((name) => name.endsWith(".js"))) {
    const source = await readFile(new URL(file, folder), "utf8");
    for (const match of source.matchAll(/\bt\("([^"]+)"/g)) used.add(match[1]);
  }
  assert.deepEqual([...used].filter((key) => !(key in LANGUAGE_FILES.en.texts)), []);
});

test("a column sorts numbers as numbers, addresses naturally, and empty cells last", () => {
  const rows = [{ value: "9.5 °C" }, { value: "" }, { value: "21 °C" }, { value: "-3 °C" }];
  const columns = { value: (row) => row.value };
  assert.deepEqual(sortRows(rows, { key: "value", descending: false }, columns).map((row) => row.value), ["-3 °C", "9.5 °C", "21 °C", ""]);
  assert.deepEqual(sortRows(rows, { key: "value", descending: true }, columns).map((row) => row.value), ["21 °C", "9.5 °C", "-3 °C", ""]);
  const addresses = [{ value: "10.0.0.20" }, { value: "10.0.0.3" }];
  assert.equal(sortRows(addresses, { key: "value", descending: false }, columns)[0].value, "10.0.0.3");
  assert.equal(sortRows(rows, null, columns), rows, "no column chosen keeps the device's order");
});

test("a column head goes ascending, descending, then back to the device's order", () => {
  const first = nextSort(null, "name");
  assert.deepEqual(first, { key: "name", descending: false });
  const second = nextSort(first, "name");
  assert.deepEqual(second, { key: "name", descending: true });
  assert.equal(nextSort(second, "name"), null);
  assert.deepEqual(nextSort(second, "value"), { key: "value", descending: false });
});

test("an answer that is no JSON gives a sentence, not a parser's message", async () => {
  const { api } = await import("../../web/api.js");
  const real = globalThis.fetch;
  globalThis.fetch = async () => ({ ok: false, status: 404, text: async () => "Not found" });
  try {
    await assert.rejects(api.events(), (error) => /\/api\/events/.test(error.message) && !/JSON/.test(error.message));
  } finally {
    globalThis.fetch = real;
  }
});

test("a site file that is no JSON gives a sentence, not the engine's parser message", async () => {
  const { siteFromFile } = await import("../../web/site-actions.js");
  assert.deepEqual(siteFromFile('{ "name": "Plant B" }'), { name: "Plant B" });
  setLanguage("de");
  assert.throws(
    () => siteFromFile("<html>no site file</html>"),
    (error) => /Anlagendatei/.test(error.message) && !/unexpected|token|position/i.test(error.message),
  );
  setLanguage("en");
});

test("the note about a taken port is in the language of the page, also its reason", () => {
  setLanguage("de");
  const text = noteText({ code: "port_in_use", reason: "another program uses 192.168.10.240:47808", text: "x" });
  setLanguage("en");
  assert.match(text, /ein anderes Programm nutzt 192\.168\.10\.240:47808/);
  assert.doesNotMatch(text, /another program/);
});

test("without path names, schedules, trend logs and other objects get a group per type", () => {
  const points = [point(0, 1, "a"), point(17, 1, "s"), point(20, 1, "t"), point(20, 2, "u"), point(2, 1, "v")];
  const groups = groupPoints(points);
  assert.deepEqual(groups.map((group) => group.key), ["kind:inputs", "kind:values", "kind:type:17", "kind:type:20"]);
  assert.equal(groups.at(-1).items.length, 2);
});

/** Every .rs file of the program, so that a message in a new file is not missed. */
async function rustSources(folder = "src") {
  const { readdir } = await import("node:fs/promises");
  const found = [];
  for (const entry of await readdir(folder, { withFileTypes: true })) {
    const path = `${folder}/${entry.name}`;
    if (entry.isDirectory()) found.push(...(await rustSources(path)));
    else if (entry.name.endsWith(".rs")) found.push(path);
  }
  return found;
}

test("every message the server sends for a wrong request has a translation", async () => {
  const { readFile } = await import("node:fs/promises");
  const { readdirSync } = await import("node:fs");
  const text = (file) => readFile(file, "utf8");
  const refusals = (await Promise.all((await rustSources()).map(text))).join("\n");
  const messages = [...refusals.matchAll(/ApiError::(?:bad_request|conflict|device)\(\s*(?:format!\(\s*)?"((?:[^"\\]|\\.)*)"/gs)].map((match) => match[1]);
  // these modules check what was typed and hand their sentence to ApiError unchanged
  const checks = ["src/write.rs", "src/connection.rs", "src/interfaces.rs", "src/record.rs"];
  const checked = (await Promise.all(checks.map(text))).join("\n");
  messages.push(...[...checked.matchAll(/(?:Err|bail!|format!)\(\s*"((?:[^"\\]|\\.)*)"/gs)].map((match) => match[1]));
  assert.ok(messages.length > 40, `found ${messages.length}`);
  setLanguage("de");
  const untranslated = messages
    // nothing of its own: the message comes from somewhere else and is translated there
    .filter((message) => !/^(\s*\{[^}]*\})+\s*$/.test(message))
    // "{text} is not {what}" builds its sentence from a word, and those sentences are messages of
    // their own; "another program uses {interface}:{port}" reaches the page inside a note
    .filter((message) => !/\{\w+\}\.?$/.test(message))
    // as Rust reads the literal: a line continuation eats the line break and the indent behind it
    .map((message) => message.replace(/\\\s+/g, "").replace(/\{[^}]*\}/g, "7").replace(/\\"/g, '"'))
    .filter((message) => errorText(message) === message);
  setLanguage("en");
  assert.deepEqual(untranslated, []);
  assert.ok(readdirSync("web/lang").length === 5);
});

test("a flat list stops at a thousand rows and offers the rest", () => {
  const state = baseState();
  state.area = "points";
  state.device = { instance: 7, address: "10.0.0.7", name: "AHU", vendor: "V", model: "M" };
  state.points = Array.from({ length: 1500 }, (_, index) => point(0, index, `p${index}`));
  state.pointSort = { key: "name", descending: false };
  const short = viewPoints(state).html;
  assert.equal((short.match(/data-point=/g) ?? []).length, 1000);
  assert.match(short, /data-act="all-rows"/);
  state.allRows = true;
  assert.equal((viewPoints(state).html.match(/data-point=/g) ?? []).length, 1500);
});

test("figures weigh a value by how long it held, and gaps do not count", () => {
  const found = figures([[0, 10], [1000, 20], [2000, null], [4000, 40]]);
  assert.equal(found.low, 10);
  assert.equal(found.high, 40);
  assert.equal(found.last, 40);
  assert.equal(found.gaps, 1);
  assert.equal(found.mean, (10 * 1000 + 20 * 3000) / 4000);
  assert.equal(figures([]).mean, null);
});

test("a stepped series tells its share of time on and its changes", () => {
  const found = figures([[0, 0], [1000, 1], [4000, 0], [5000, 0]], true);
  assert.equal(found.onShare, 3000 / 5000);
  assert.equal(found.changes, 2);
});

test("a window keeps what is inside it; only one unit shares an axis", () => {
  assert.deepEqual(inWindow([[1, 1], [5, 2], [9, 3]], { from: 2, to: 8 }), [[5, 2]]);
  const groups = byUnit([{ key: "a", units: "°C" }, { key: "b", units: "%" }, { key: "c", units: "°C" }, { key: "d", stepped: true }]);
  assert.deepEqual(groups.map((group) => group.map((item) => item.key)), [["a", "c"], ["b"], ["d"]]);
});

test("thinning keeps spikes and gaps", () => {
  const samples = Array.from({ length: 10000 }, (_, index) => [index, index === 4321 ? 999 : index === 7000 ? null : 1]);
  const few = thinned(samples, 100);
  assert.ok(few.length < 400);
  assert.ok(few.some(([, value]) => value === 999));
  assert.ok(few.some(([, value]) => value === null));
  assert.equal(thinned(samples.slice(0, 50), 100).length, 50);
});

test("the counts above the list count the chosen plant part, and the list says when it was read", () => {
  const state = baseState();
  const point = (name, instance, status = "") => ({ type: 0, instance, object: `Analog input ${instance}`, name, value: "1", units: null, status, state_texts: [], error: null });
  state.device = { instance: 7, address: "10.0.0.7" };
  state.points = [point("A.01/x", 1, "fault"), point("A.01/y", 2), point("B.01/z", 3)];
  state.node = "part:A.01";
  state.readAt[7] = Date.now() - 12 * 60000;
  const { html } = viewPoints(state);
  assert.match(html, /data-view="all"[^>]*>[^<]*<span class="n ">2</);
  assert.match(html, /chip warn">Read .* 12 min ago/);
  state.readAt[7] = Date.now();
  assert.doesNotMatch(viewPoints(state).html, /min ago/);
});

test("axis labels take as many decimals as different values need", () => {
  assert.deepEqual(axisTexts([0.0851, 0.0942]), ["0.085", "0.094"]);
  assert.deepEqual(axisTexts([18, 22.5]), ["18", "22.5"]);
});

test("names with a path of their own for every point are grouped a level higher", () => {
  // as K&P names them: one path per point, the attribute last
  const names = ["001/00/00/P.01/CAI/b", "001/00/00/P.17/CAO/Y", "001/00/00/P.02/CAI/b", "001/00/00/P.18/CAO/Y",
    "002/00/00/P.01/CDI/k", "002/00/00/P.21/CDO/K", "002/00/00/P.04/CDI/k", "000/02/01/S116.01", "000/02/01/S118.01", "000/02/01/S118.02"];
  const groups = groupPoints(names.map((name, index) => point(0, index + 1, `/31/${name}`)));
  assert.deepEqual(groups.map((group) => group.title), ["001/00/00", "002/00/00", "000/02/01"]);
  assert.deepEqual(groups[0].items.map((item) => item.name), ["P.01/CAI/b", "P.17/CAO/Y", "P.02/CAI/b", "P.18/CAO/Y"]);
  assert.equal(groups[2].items[0].name, "S116.01");
});

test("each measurement gets a dot, each one without answer a red tick", () => {
  const { marksSvg: marks } = chartModule;
  const samples = [[0, 1], [1000, null], [2000, 1]];
  const s = scale(samples);
  const svg = marks(samples, s, 100);
  assert.equal((svg.match(/class="mark"/g) ?? []).length, 2);
  assert.equal((svg.match(/class="gap-mark"/g) ?? []).length, 1);
  const many = Array.from({ length: 200 }, (_, index) => [index * 1000, 1]);
  assert.equal(marks(many, scale(many), 100), "", "too many for dots, and no gaps");
});

test("the list search knows an instance, types and value conditions besides words", async () => {
  const { matchesQuery, parseQuery } = await import("../../web/point-query.js");
  setLanguage("en");
  const supply = point(0, 2796214, "Supply air temp", { value: "21.4 °C" });
  const damper = point(1, 3, "Damper", { value: "55 %" });
  const fan = point(3, 4, "Fan", { value: "active" });
  const find = (text) => [supply, damper, fan].filter((found) => matchesQuery(found, parseQuery(text))).map((found) => found.name);
  assert.deepEqual(find("#2796214"), ["Supply air temp"]);
  assert.deepEqual(find("type:ao,bi"), ["Damper", "Fan"]);
  assert.deepEqual(find("value:>50"), ["Damper"]);
  assert.deepEqual(find("value:<=21,4"), ["Supply air temp"]);
  assert.deepEqual(find("value:active"), ["Fan"]);
  assert.deepEqual(find("air type:ai"), ["Supply air temp"]);
  assert.deepEqual(find("type:"), [], "a prefix without anything after it is a word");
  assert.deepEqual(parseQuery("Supply #12").words, ["supply"]);
});

test("a COV line goes on at its last value until now, without a mark there", () => {
  const { heldOn } = chartModule;
  const samples = [[1000, 20], [2000, 21]];
  assert.deepEqual(heldOn(samples, 5000), [[1000, 20], [2000, 21], [5000, 21]]);
  assert.equal(heldOn(samples, null), samples, "a polled point ends at its last measurement");
  assert.equal(heldOn([[1000, 20], [2000, null]], 5000).length, 2, "after a gap nothing is held");
  const html = chartModule.chartHtml("held", { samples, stepped: false, units: "°C", label: "x", until: 5000 }, "");
  assert.equal((html.match(/class="mark"/g) ?? []).length, 2);
});

test("properties by their meaning: flags, acknowledgements, limits, stamps", async () => {
  const { propertySectionsHtml } = await import("../../web/properties.js");
  const state = {
    ...baseState(),
    device: { instance: 21, address: "127.0.0.3:47809" },
    propertyFilter: "",
    propertyNames: "plain",
    propertiesOther: false,
    changedProperties: {},
    editProperty: null,
    writeUnlocked: false,
    properties: [
      { property: "status-flags", value: "overridden", data_type: "bits", bits: [false, false, true, false] },
      { property: "acked-transitions", value: "101", data_type: "bits", bits: [true, false, true] },
      { property: "limit-enable", value: "01", data_type: "bits", bits: [false, true] },
      { property: "high-limit", value: "100 °C", data_type: "real" },
      { property: "low-limit", value: "0 °C", data_type: "real" },
      { property: "out-of-service", value: "false", data_type: "boolean" },
      { property: "event-time-stamps", value: "2026-11-02 14:03:00, never, never", data_type: "other", items: ["2026-11-02 14:03:00", "never", "never"] },
      { property: "vendor-thing", value: "7", data_type: "unsigned" },
    ],
  };
  const object = { type: 0, instance: 205 };
  const html = propertySectionsHtml(state, object);
  assert.match(html, /chip warn">Overridden/);
  assert.match(html, /Not acknowledged: Fault/);
  assert.match(html, /Low limit<\/dt><dd>0 °C <span class="unit">· not monitored/);
  assert.doesNotMatch(html, /High limit<\/dt><dd>100 °C <span class="unit">· not monitored/);
  assert.match(html, /Out of service<\/dt><dd>No/);
  assert.match(html, /Alarm<\/span><span>2026-11-02 14:03:00[\s\S]*Fault<\/span><span><span class="blank">never/);
  // a property outside the sections waits behind its button, and a search opens it
  assert.match(html, /More properties \(1\)/);
  assert.doesNotMatch(html, /Vendor thing/);
  state.propertyFilter = "vendor";
  assert.match(propertySectionsHtml(state, object), /Vendor thing<\/dt><dd>7/);
  // the BACnet view shows the names of the standard and what the device sent
  state.propertyFilter = "";
  state.propertyNames = "bacnet";
  assert.match(propertySectionsHtml(state, object), /acked-transitions · bits<\/span><\/dt><dd><span class="mono">101/);
});

test("chart axes: time marks at round times, states instead of numbers", async () => {
  const { timeTicks, valueLevels, valueLabel } = await import("../../web/chart.js");
  // an hour from 10:03: a mark every 10 minutes, on the minute
  const from = new Date(2026, 10, 2, 10, 3, 20).getTime();
  const ticks = timeTicks(from, from + 60 * 60 * 1000);
  assert.deepEqual(ticks.map((tick) => new Date(tick.ms).getMinutes()), [10, 20, 30, 40, 50, 0]);
  assert.ok(ticks.every((tick) => new Date(tick.ms).getSeconds() === 0));
  // a minute: every 10 seconds, with seconds in the label
  const short = timeTicks(from, from + 60 * 1000);
  assert.equal(short.length, 7);
  assert.match(short[0].label, /10:03:20/);
  // a binary point is off or on, not −1 to 1; a multi-state point by its states
  assert.deepEqual(valueLevels("binary", 1, 1).labels, ["inactive", "active"]);
  assert.deepEqual(valueLevels("multi", 2, 3).labels, ["state 1", "state 2", "state 3"]);
  assert.deepEqual(valueLevels(null, 5, 5), { low: 4, high: 6, levels: [4, 6], labels: ["4", "6"] });
  assert.equal(valueLabel("binary", 1), "active");
  assert.equal(valueLabel(null, 21.456, "°C"), "21.46 °C");
});

test("an adapter read again keeps its place, also with a new address", () => {
  const cable = { name: "eth0", ip: "10.0.0.2", default: true };
  const vpn = { name: "tun0", ip: "192.168.10.222", default: false };
  assert.equal(chooseAdapter([cable, vpn], { ip: "192.168.10.222" }), vpn);
  assert.equal(chooseAdapter([cable, { ...vpn, ip: "192.168.10.5" }], vpn).ip, "192.168.10.5");
  // gone: the system's choice
  assert.equal(chooseAdapter([cable], vpn), cable);
  assert.equal(chooseAdapter([], vpn), null);
});
