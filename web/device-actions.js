// Searching, choosing and opening devices.

import { api } from "./api.js";
import { t } from "./i18n.js";
import { saveMemory } from "./memory.js";
import { loadSitePoints, loadSites } from "./site-actions.js";
import { selectPoint } from "./object-actions.js";
import { go, render, state, toast } from "./store.js";

const SCAN_POLL_MS = 400;
const READ_POLL_MS = 400;
/** Device properties shown in plain words beside the device list. */
const DEVICE_FACTS = [
  ["vendor-name", "fact.vendor"], ["model-name", "fact.model"], ["firmware-revision", "fact.firmware"],
  ["application-software-version", "fact.software"], ["location", "fact.location"], ["description", "description"],
  ["system-status", "fact.systemStatus"], ["local-date", "fact.date"], ["local-time", "fact.time"],
  ["protocol-revision", "fact.protocol"], ["segmentation-supported", "fact.segmentation"], ["max-apdu-length-accepted", "fact.maxApdu"],
];
const DEVICE_TYPE = 8;

/** Adapter, port and BBMD as the API takes them. */
export function networkSettings() {
  const settings = { interface: state.adapter.ip };
  if (state.settings.bbmd.trim()) settings.bbmd = state.settings.bbmd.trim();
  const port = Number(state.settings.port);
  if (port && port !== 47808) settings.port = port;
  return settings;
}

/**
 * A search over IP leaves the trunk bacnetio is on. With values held it asks first, as leaving the
 * trunk does: false then, and the search waits for the answer.
 */
function mayLeaveTrunk() {
  if (!state.mstp?.trunk || !state.held.length) return true;
  state.mstp.leaving = true;
  state.menu = null;
  render();
  return false;
}

/** Whether the server left the trunk for the connection over IP: the sidebar and the log say so. */
async function checkTrunk() {
  if (!state.mstp?.trunk) return;
  try {
    state.mstp.trunk = await api.mstpTrunk();
    if (!state.mstp.trunk) state.log = await api.log();
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

export async function find() {
  if (!state.adapter || !mayLeaveTrunk()) return;
  const settings = { ...networkSettings(), sweep: true };
  if (state.settings.subnet.trim()) settings.subnet = state.settings.subnet.trim();
  if (state.settings.idFrom.trim()) settings.id_from = Number(state.settings.idFrom);
  if (state.settings.idTo.trim()) settings.id_to = Number(state.settings.idTo);
  saveMemory(state);
  const started = await runSearch(() => api.startScan(settings));
  await checkTrunk();
  return started;
}

/** A new search: the old devices go, the workspace shows the new ones as they come. */
export async function runSearch(start) {
  state.scan = { state: "running", phase: "who_is", found: 0, asked: 0, total: 0, devices: [] };
  state.selectedDevice = null;
  state.device = null;
  state.points = null;
  state.sitePoints = {};
  state.node = null;
  go("workspace");
  try {
    await start();
  } catch (error) {
    state.scan = { state: "failed", error: error.message };
    render();
    return false;
  }
  pollScan();
  return true;
}

export async function pollScan() {
  try {
    state.scan = await api.scan();
  } catch (error) {
    state.scan = { state: "failed", error: error.message };
  }
  render();
  if (state.scan.state === "running") setTimeout(pollScan, SCAN_POLL_MS);
  // a finished search is kept as a site, perhaps a new one
  else loadSites();
}

export async function locate() {
  if (!mayLeaveTrunk()) return;
  try {
    const request = { ...networkSettings(), address: state.settings.locate.trim() };
    if (state.settings.locateNetwork.trim()) {
      request.network = Number(state.settings.locateNetwork);
      request.mac = state.settings.locateMac.trim();
      request.instance = Number(state.settings.locateId);
    }
    const device = await api.locate(request);
    const devices = (state.scan.devices ?? []).filter((known) => known.instance !== device.instance);
    state.scan = { state: "done", notes: [], routers: [], refused: [], ...state.scan, devices: [...devices, device] };
    state.menu = null;
    openDevice(device.instance);
  } catch (error) {
    toast(error.message, true);
  }
  await checkTrunk();
}

export async function selectDevice(instance) {
  if (state.selectedDevice === instance) return;
  state.selectedDevice = instance;
  state.deviceFacts = null;
  state.deviceAlarms = null;
  state.alarmsAll = false;
  state.requestLog = false;
  render();
  loadAlarms(instance);
  try {
    const properties = await api.properties(instance, DEVICE_TYPE, instance);
    if (state.selectedDevice !== instance) return;
    state.deviceFacts = DEVICE_FACTS.flatMap(([property, key]) => {
      const found = properties.find((item) => item.property === property && item.value);
      return found ? [[t(key), found.value]] : [];
    });
  } catch (error) {
    if (state.selectedDevice === instance) state.deviceFacts = [[t("readProblem"), error.message]];
  }
  render();
}

/** The device's active alarms, beside its facts; a device without GetEventInformation says so. */
async function loadAlarms(instance) {
  let alarms;
  try {
    alarms = await api.alarms(instance);
  } catch (error) {
    alarms = { error: error.message };
  }
  if (state.selectedDevice !== instance) return;
  state.deviceAlarms = alarms;
  render();
}

/** Names and units of the points read, so the held values and the log can name them. */
function rememberNames(instance, points) {
  for (const point of points) {
    const key = `${instance}:${point.type}:${point.instance}`;
    if (point.name) state.objectNames[key] = point.name;
    if (point.units) state.objectUnits[key] = point.units;
  }
}

/** Opens the device's points; `fresh` asks the device again even if it was read before. */
export async function openDevice(instance, fresh = false) {
  const device = state.scan.devices?.find((found) => found.instance === instance);
  if (!device) return;
  if (state.device?.instance !== instance) {
    state.filter = "";
    state.pointView = "all";
    state.pointTypes = [];
    state.allRows = false;
    state.groupsDefault = null;
    state.node = null;
    state.winners = {};
    state.groupsOpen = {};
    state.selectedPoint = null;
    state.object = null;
    state.points = null;
    state.compare = null;
    state.selecting = false;
    state.selected = [];
    state.changeMany = null;
  }
  state.device = device;
  state.reading = true;
  state.error = null;
  saveMemory(state);
  go("workspace");
  selectDevice(instance);
  try {
    const last = fresh ? null : await api.lastPoints(instance);
    if (last) {
      state.points = last.points;
      state.pointNotes = last.notes;
      rememberNames(instance, last.points);
    } else {
      state.readingDevice = instance;
      await api.startRead(instance);
      await followReading(instance);
    }
    if (state.device?.instance === instance) state.reference = await api.reference(instance);
    await loadSitePoints();
  } catch (error) {
    state.error = error.message;
  }
  state.reading = false;
  state.readProgress = null;
  render();
}

const pause = (ms) => new Promise((done) => setTimeout(done, ms));

/** Shows the points as the background reading brings them, until it is done. */
async function followReading(instance) {
  for (;;) {
    const view = await api.readProgress(instance);
    if (state.device?.instance !== instance) return;
    state.readProgress = view;
    // a new reading keeps showing the old points until its object list is known
    if (view.points.length) {
      state.points = view.points;
      rememberNames(instance, view.points);
    }
    if (view.state === "failed") throw new Error(view.error);
    if (view.state !== "running") {
      state.pointNotes = view.notes;
      state.readAt[instance] = Date.now();
      return;
    }
    render();
    await pause(READ_POLL_MS);
  }
}

/** This computer's local time as devices take it. BACnet counts weekdays from Monday = 1. */
export function localClock() {
  const now = new Date();
  const two = (number) => String(number).padStart(2, "0");
  return {
    date: `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`,
    time: `${two(now.getHours())}:${two(now.getMinutes())}:${two(now.getSeconds())}`,
    weekday: now.getDay() === 0 ? 7 : now.getDay(),
  };
}

/** Opens a device and a point on it, from the problems, the search or the watch list. */
export async function openPointAt(device, key) {
  await openDevice(device);
  if (key) await selectPoint(key);
}

/** Shows or hides the request log of the chosen device. */
export async function toggleRequests() {
  state.requestLog = !state.requestLog;
  render();
  if (!state.requestLog) return;
  try {
    state.requests = (await api.readProgress(state.selectedDevice)).requests;
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

export async function saveReference() {
  try {
    state.reference = await api.saveReference(state.device.instance);
    toast(t("ref.saved"));
  } catch (error) {
    toast(error.message, true);
  }
  render();
}
