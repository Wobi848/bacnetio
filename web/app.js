// Navigation and the wiring of clicks, typing and keys to actions. The areas are addressed by the
// hash (#/devices), so the browser's back button and a reload keep the place.

import { loadAdapters, watchAdapters } from "./adapter-actions.js";
import { api } from "./api.js";
import { readBbmdTables } from "./bbmd-actions.js";
import { compareTime, hideCompareCrosshair, hideCrosshair, showCompareCrosshair, showCrosshair } from "./chart.js";
import { acknowledgeAlarm, askAcknowledge, askWhoHas, changeInstance, readWinners, renameDevice, runCommand } from "./device-commands.js";
import { find, locate, openDevice, openPointAt, pollScan, saveReference, selectDevice, toggleRequests } from "./device-actions.js";
import { deleteSite, exportSiteEde, importSite, loadSitePoints, loadSites, openSite, readAllDevices, renameSite, stopRunning } from "./site-actions.js";
import { setLanguage, t } from "./i18n.js";
import { toggleLiveList } from "./live-list.js";
import { loadMemory, loadPropertyNames, loadTreeView, loadWatch, saveMemory, savePropertyNames, saveTreeView } from "./memory.js";
import { installColumns } from "./columns.js";
import { installPanels } from "./panels.js";
import { installDragToggle } from "./drag-toggle.js";
import { editDay, editProperty, releaseHeld, releaseHere, releaseOther, selectPoint, setClock, showTechnical, writeDay, writeProperty, writeValue } from "./object-actions.js";
import { installPlugin, installPluginBridge, loadPlugins, pluginsMayAsk, removePlugin, switchPlugin } from "./plugin-actions.js";
import { pollRecording, recordHere, recordPoint, recordingRequest, removeRecorded, setRecordMode } from "./recording-actions.js";
import { setAccent, setBackground, setTheme } from "./settings.js";
import { nextSort } from "./sorting.js";
import { checkForUpdate, setUpdateCheck } from "./update.js";
import { TABLE_FILTERS, noTableFilter } from "./table-filter.js";
import { AREAS, areaOfHash, forgetDeviceReads, go, refreshHeld, render, state, toast } from "./store.js";
import { loadStructure } from "./structure-actions.js";
import { pollWatch, unwatch, unwatchHere, unwatchPoint, watchHere, watchPoint } from "./watch-actions.js";
import { isRecorded, isWatched, pointKey } from "./format.js";

/** Fields whose typing goes straight into the state. */
const FIELDS = {
  bbmd: (value) => { state.settings.bbmd = value; },
  port: (value) => { state.settings.port = value; },
  subnet: (value) => { state.settings.subnet = value; },
  "id-from": (value) => { state.settings.idFrom = value; },
  "id-to": (value) => { state.settings.idTo = value; },
  "locate-address": (value) => { state.settings.locate = value; },
  "locate-network": (value) => { state.settings.locateNetwork = value; },
  "locate-mac": (value) => { state.settings.locateMac = value; },
  "locate-id": (value) => { state.settings.locateId = value; },
};
/** Fields that filter a list and redraw it while typing. */
/** Which sort each table keeps. */
const SORTS = { devices: "deviceSort", points: "pointSort", problems: "problemSort", watch: "watchSort" };

const FILTERS = {
  filter: (value) => { state.filter = value; },
  "device-filter": (value) => { state.deviceFilter = value; },
  "property-filter": (value) => { state.propertyFilter = value; },
  "site-search": (value) => {
    if (!state.siteSearch && value) loadSitePoints();
    state.siteSearch = value;
    state.whoHas = null;
  },
};

/** The chosen device's name as last read, to start a new one from. */
const deviceNameNow = () => (state.scan.devices ?? []).find((device) => device.instance === state.selectedDevice)?.name ?? "";

/** Goes to an area, with what that area needs loaded. */
function enter(area) {
  go(area);
  if (area === "problems") loadSitePoints();
  if (area === "watch") pollWatch();
  if (area === "sites") loadSites();
}

/** Back to the site as a whole: the device list in the table. */
function showSite() {
  state.device = null;
  state.points = null;
  state.selectedPoint = null;
  state.object = null;
  state.node = null;
  state.selectedDevice = null;
  saveMemory(state);
  go("workspace");
}

/** Opens a device's points and then the plant view it carries itself (Structured View). */
async function showDevice(instance, fresh = false) {
  await openDevice(instance, fresh);
  await loadStructure(instance, fresh);
}

async function chooseNode(element) {
  const device = Number(element.dataset.device);
  if (state.device?.instance !== device) await showDevice(device);
  state.node = element.dataset.node;
  render();
}

function leaveSite() {
  showSite();
  toast(t("leave.done"));
}

const listedPoint = (key) => state.points.find((point) => pointKey(point) === key);

const actions = {
  find: () => { forgetDeviceReads(); find(); },
  locate: () => locate(),
  network: () => { state.menu = state.menu === "network" ? null : "network"; render(); document.getElementById("subnet")?.focus(); },
  "locate-menu": () => { state.menu = state.menu === "locate" ? null : "locate"; render(); document.getElementById("locate-address")?.focus(); },
  "close-menu": () => { state.menu = null; render(); },
  "toggle-head": () => { state.headOpen = !state.headOpen; render(); },
  "focus-adapter": () => document.getElementById("adapter")?.focus(),
  go: (element) => enter(element.dataset.area),
  "tree-site": () => showSite(),
  "tree-device": (element) => {
    state.node = null;
    state.selectedPoint = null;
    state.object = null;
    showDevice(Number(element.dataset.device));
  },
  "tree-node": (element) => chooseNode(element),
  "tree-toggle": (element) => {
    const device = element.dataset.device;
    state.treeOpen[device] = element.getAttribute("aria-expanded") !== "true";
    render();
  },
  "device-details": () => {
    state.selectedPoint = null;
    state.object = null;
    state.detailOpen = true;
    render();
  },
  "day-edit": (element) => editDay(Number(element.dataset.day)),
  "day-cancel": () => { state.editDay = null; render(); },
  "day-write": () => writeDay(),
  "all-rows": () => { state.allRows = true; render(); },
  "live-list": () => toggleLiveList(),
  "more-menu": () => { state.menu = state.menu === "more" ? null : "more"; render(); },
  "types-menu": () => { state.menu = state.menu === "types" ? null : "types"; render(); },
  "groups-all": (element) => {
    state.groupsOpen = {};
    state.groupsDefault = element.dataset.open === "true";
    state.menu = null;
    render();
  },
  "property-names": (element) => {
    state.propertyNames = element.dataset.names;
    savePropertyNames(state.propertyNames);
    render();
  },
  "properties-other": () => {
    state.propertiesOther = !state.propertiesOther;
    render();
  },
  "tree-view": (element) => {
    state.treeView = element.dataset.view;
    saveTreeView(state.treeView);
    render();
  },
  "live-mode": (element) => {
    state.liveModes[`${state.device.instance}:${state.object.type}:${state.object.instance}`] = element.dataset.mode;
    state.liveSource = null;
    render();
  },
  "toggle-commands": () => { state.commandsOpen = !state.commandsOpen; render(); },
  "read-winners": () => readWinners(),
  "bbmd-read": () => readBbmdTables(),
  "property-edit": (element) => editProperty(element.dataset.property),
  "property-cancel": () => { state.editProperty = null; render(); },
  "property-write": () => writeProperty(),
  "site-open": (element) => { forgetDeviceReads(); openSite(state.sites[Number(element.dataset.index)].name); },
  "site-ask-delete": (element) => { state.deletingSite = Number(element.dataset.index); render(); },
  "site-delete": () => deleteSite(state.sites[state.deletingSite].name),
  "site-keep": () => { state.deletingSite = null; render(); },
  "site-rename": (element) => {
    const index = Number(element.dataset.index);
    state.renamingSite = index;
    state.siteTitle = state.sites[index].title ?? "";
    render();
    document.getElementById("site-title")?.focus();
  },
  "site-rename-cancel": () => { state.renamingSite = null; render(); },
  // a new site starts with a search; the network form says where
  "site-new": () => { enter("workspace"); actions.network(); },
  "export-menu": () => { state.menu = state.menu === "export" ? null : "export"; render(); },
  "stop-running": () => stopRunning(),
  "read-all": () => readAllDevices().then((failed) => { if (failed) toast(t("ede.siteFailed", { n: failed }), true); }),
  "choose-priority": (element) => { state.writePriority = Number(element.dataset.priority); render(); },
  "confirm-yes": () => (state.confirm.kind === "write" ? writeValue(true) : releaseHere(state.confirm.priority, true)),
  "confirm-no": () => { state.confirm = null; render(); },
  "watch-add": () => watchHere(),
  "quick-watch": (element) => {
    const point = listedPoint(element.dataset.key);
    if (isWatched(state, state.device.instance, point)) unwatchPoint(state.device.instance, point);
    else { watchPoint(state.device.instance, point); toast(t("watch.added")); }
  },
  "quick-record": (element) => {
    const point = listedPoint(element.dataset.key);
    recordPoint(state.device.instance, point, !isRecorded(state, state.device.instance, point));
  },
  "watch-remove-here": () => unwatchHere(),
  "watch-remove": (element) => unwatch(Number(element.dataset.index)),
  leave: () => {
    if (state.held.length) { state.leaving = true; render(); return; }
    leaveSite();
  },
  "leave-stay": () => { state.leaving = false; render(); },
  "leave-release": async () => {
    await releaseHeld([...state.held]);
    state.leaving = false;
    if (state.held.length) { render(); return; }
    leaveSite();
  },
  "open-device": (element) => showDevice(Number(element.dataset.id)),
  "close-detail": () => { state.detailOpen = false; render(); },
  reload: () => showDevice(state.device.instance, true),
  "ref-save": () => saveReference(),
  sort: (element) => {
    const list = SORTS[element.dataset.list];
    state[list] = nextSort(state[list], element.dataset.key);
    render();
  },
  "point-view": (element) => { state.pointView = element.dataset.view; render(); },
  "point-type": (element) => {
    const type = Number(element.dataset.type);
    state.pointTypes = state.pointTypes.includes(type) ? state.pointTypes.filter((chosen) => chosen !== type) : [...state.pointTypes, type];
    render();
  },
  "filter-remove": (element) => {
    const kind = element.dataset.kind;
    if (kind === "search") state.filter = "";
    if (kind === "view") state.pointView = "all";
    if (kind === "type") state.pointTypes = state.pointTypes.filter((type) => type !== Number(element.dataset.value));
    render();
  },
  "alarms-all": () => { state.alarmsAll = !state.alarmsAll; render(); },
  "table-view": (element) => { state[TABLE_FILTERS[element.dataset.list]].view = element.dataset.view; render(); },
  "table-clear": (element) => { state[TABLE_FILTERS[element.dataset.list]] = noTableFilter(); render(); },
  "filters-clear": () => {
    state.filter = "";
    state.pointView = "all";
    state.pointTypes = [];
    render();
  },
  "ask-write": () => { state.asking = true; render(); },
  "ask-properties": () => { state.asking = "properties"; render(); },
  "ask-clock": () => { state.asking = "clock"; render(); },
  "ask-command": (element) => { state.asking = `cmd:${element.dataset.command}`; render(); },
  "unlock-clock": () => { state.writeUnlocked = true; render(); },
  "unlock-command": () => { state.writeUnlocked = true; render(); },
  "run-command": (element) => runCommand(element.dataset.command),
  "unlock-ack": (element) => {
    state.writeUnlocked = true;
    acknowledgeAlarm(Number(element.dataset.type), Number(element.dataset.instance));
  },
  "alarm-ack": (element) => askAcknowledge(Number(element.dataset.type), Number(element.dataset.instance)),
  "who-has": () => askWhoHas(),
  "ask-instance": () => { state.asking = "instance"; render(); },
  "ask-name": () => { state.asking = "name"; state.newName = deviceNameNow(); render(); },
  "unlock-name": () => { state.writeUnlocked = true; render(); },
  "change-name": () => renameDevice(),
  "unlock-instance": () => { state.writeUnlocked = true; render(); },
  "change-instance": () => changeInstance(),
  "cancel-write": () => { state.asking = false; render(); },
  unlock: () => { state.writeUnlocked = true; state.asking = false; render(); },
  write: () => writeValue(),
  release: (element) => releaseHere(Number(element.dataset.priority)),
  "release-other": (element) => releaseOther(Number(element.dataset.priority)),
  "release-held": (element) => releaseHeld([state.held[Number(element.dataset.index)]]),
  "release-all": () => releaseHeld([...state.held]),
  technical: () => showTechnical(),
  "set-clock": () => setClock(),
  requests: () => toggleRequests(),
  "site-ede": () => exportSiteEde(),
  "rec-add": () => recordHere(true),
  "rec-remove-here": () => recordHere(false),
  "rec-remove": (element) => removeRecorded(element.dataset.key),
  "rec-mode": (element) => setRecordMode(element.dataset.key, element.dataset.mode),
  "rec-interval": (element) => {
    state.recordInterval = Number(element.dataset.seconds);
    if (state.recording?.running) recordingRequest(() => api.recordStart(state.recordInterval));
    else render();
  },
  "rec-view": (element) => { state.recView = element.dataset.view; render(); },
  "rec-window": (element) => {
    state.recWindow = element.dataset.minutes ? Number(element.dataset.minutes) : null;
    state.recZoom = null;
    render();
  },
  "rec-zoom-out": () => { state.recZoom = null; render(); },
  "rec-clear": () => recordingRequest(() => api.recordClear()),
  "rec-start": () => recordingRequest(() => api.recordStart(state.recordInterval)),
  "rec-stop": () => recordingRequest(() => api.recordStop()),
  language: (element) => { setLanguage(element.dataset.code); render(); },
  theme: (element) => { setTheme(element.dataset.theme); render(); },
  "update-check": (element) => setUpdateCheck(element.dataset.on === "true"),
  "update-now": () => checkForUpdate(true),
  "plugin-switch": (element) => switchPlugin(element.dataset.id, element.dataset.on === "true"),
  "plugin-remove": (element) => { state.pluginRemoving = element.dataset.id; render(); },
  "plugin-remove-yes": (element) => removePlugin(element.dataset.id),
  "plugin-remove-no": () => { state.pluginRemoving = null; render(); },
  "plugin-replace-yes": () => installPlugin(state.pluginReplacing.page, true),
  "plugin-replace-no": () => { state.pluginReplacing = null; render(); },
  accent: (element) => { setAccent(element.dataset.accent); render(); },
  background: (element) => { setBackground(element.dataset.scheme, element.dataset.background); render(); },
};

async function openFound(row) {
  state.siteSearch = "";
  const device = Number(row.dataset.openDevice);
  await openPointAt(device, row.dataset.openPoint);
  await loadStructure(device);
}

function toggleGroup(row) {
  state.groupsOpen[row.dataset.group] = row.getAttribute("aria-expanded") !== "true";
  render();
}

document.addEventListener("click", (event) => {
  const target = event.target;
  if (state.menu && !target.closest(".menu, [data-act=network], [data-act=locate-menu], [data-act=export-menu], [data-act=more-menu], [data-act=types-menu]")) {
    state.menu = null;
    render();
  }
  const element = target.closest("[data-act]");
  if (element) { actions[element.dataset.act]?.(element); return; }
  const chip = target.closest("[data-area-go]");
  if (chip) { enter(chip.dataset.areaGo); return; }
  const found = target.closest(".row[data-open-device]");
  if (found) { openFound(found); return; }
  const device = target.closest(".row[data-device]");
  if (device) { showDevice(Number(device.dataset.device)); return; }
  const point = target.closest(".row[data-point]");
  if (point) { selectPoint(point.dataset.point); return; }
  const group = target.closest(".row[data-group]");
  if (group) toggleGroup(group);
});

document.addEventListener("submit", (event) => {
  const form = event.target.closest(".site-rename");
  if (!form) return;
  event.preventDefault();
  renameSite(state.sites[Number(form.dataset.index)].name, state.siteTitle);
});

document.addEventListener("change", (event) => {
  if (event.target.id === "plugin-file") {
    if (event.target.files[0]) installPlugin(event.target.files[0]);
    return;
  }
  if (event.target.id === "site-file") {
    if (event.target.files[0]) {
      forgetDeviceReads();
      importSite(event.target.files[0]);
    }
    return;
  }
  if (event.target.id === "priority") {
    state.writePriority = Number(event.target.value);
    return;
  }
  if (event.target.id === "new-value") {
    state.newValue = event.target.value;
    return;
  }
  if (event.target.id === "table-device") {
    state[TABLE_FILTERS[event.target.dataset.list]].device = event.target.value === "" ? null : Number(event.target.value);
    render();
    return;
  }
  if (event.target.id !== "adapter") return;
  state.adapter = state.adapters[Number(event.target.value)];
  saveMemory(state);
  render();
});

document.addEventListener("input", (event) => {
  const id = event.target.id;
  if (id === "new-value") state.newValue = event.target.value;
  if (id === "new-instance") state.newInstance = event.target.value;
  if (id === "new-name") state.newName = event.target.value;
  if (id === "cmd-password") state.commandPassword = event.target.value;
  if (id === "cmd-minutes") state.commandMinutes = event.target.value;
  if (id === "property-value") state.editValue = event.target.value;
  if (id === "day-changes") state.editDayText = event.target.value;
  if (id === "site-title") state.siteTitle = event.target.value;
  if (FIELDS[id]) FIELDS[id](event.target.value);
  if (FILTERS[id]) { FILTERS[id](event.target.value); render(); }
  if (id === "table-filter") { state[TABLE_FILTERS[event.target.dataset.list]].text = event.target.value; render(); }
});

/** Moves the chosen row of the list on screen by that many rows. */
function moveSelection(step) {
  const rows = [...document.querySelectorAll(".rows .row[data-device], .rows .row[data-point]")];
  if (!rows.length) return;
  const index = rows.findIndex((row) => row.classList.contains("active"));
  const next = rows[Math.max(0, Math.min(rows.length - 1, index + step))];
  next.scrollIntoView({ block: "nearest" });
  if (next.dataset.device) selectDevice(Number(next.dataset.device));
  else selectPoint(next.dataset.point);
}

const PAGE_ROWS = 10;
const MOVE_KEYS = { ArrowDown: 1, ArrowUp: -1, PageDown: PAGE_ROWS, PageUp: -PAGE_ROWS, Home: -Infinity, End: Infinity };

const typing = () => document.activeElement?.matches?.("input, select, textarea");

/** Closes the menu, dialog, search or details on top; false when nothing was open. */
function closeTopmost() {
  if (state.menu) state.menu = null;
  else if (state.leaving) state.leaving = false;
  else if (state.renamingSite !== null) state.renamingSite = null;
  else if (state.siteSearch || document.activeElement?.id === "site-search") {
    state.siteSearch = "";
    document.activeElement.blur();
  }
  else if (state.confirm) state.confirm = null;
  else if (state.asking) state.asking = false;
  else if (state.detailOpen) state.detailOpen = false;
  else return false;
  return true;
}

/**
 * The Android back key: one step back, from the details to the device to the device list to the
 * workspace. When there is no step left, the app asks whether to leave and warns of held values.
 */
window.bacnetioBack = () => {
  if (closeTopmost()) render();
  else if (state.area === "workspace" && state.device !== null) showSite();
  else if (state.area !== "workspace") enter("workspace");
  else return JSON.stringify({ stepped: false, held: state.held.length });
  return JSON.stringify({ stepped: true, held: state.held.length });
};

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    closeTopmost();
    render();
    return;
  }
  if (event.key === "Enter" && typing()) {
    if (event.target.id.startsWith("locate-")) locate();
    else if (FIELDS[event.target.id]) find();
    return;
  }
  if (typing() || event.ctrlKey || event.metaKey || event.altKey) return;
  // 1 to 6 lie side by side on the keyboard, 0 sits apart
  if (/^[1-6]$/.test(event.key)) { enter(AREAS[Number(event.key) - 1]); return; }
  if (event.key === "/") {
    event.preventDefault();
    document.getElementById("site-search")?.focus();
    return;
  }
  if (event.key === "f") {
    event.preventDefault();
    document.getElementById("filter")?.focus();
    return;
  }
  if (MOVE_KEYS[event.key]) {
    event.preventDefault();
    moveSelection(MOVE_KEYS[event.key]);
    return;
  }
  if (event.key === "Enter" && state.area === "workspace" && !state.device && state.selectedDevice !== null) showDevice(state.selectedDevice);
});

/** Zooming by drawing a span in a compare chart: where it began, in time and on screen. */
let dragFrom = null;
let dragX = 0;

function drawZoom(event) {
  const svg = event.target.closest(".chart.compare").querySelector("svg");
  const box = svg.getBoundingClientRect();
  const scale = svg.viewBox.baseVal.width / box.width;
  const [left, right] = [dragX, event.clientX].sort((first, second) => first - second);
  const zoom = svg.querySelector(".zoom");
  zoom.hidden = false;
  zoom.setAttribute("x", (left - box.left) * scale);
  zoom.setAttribute("width", (right - left) * scale);
}

document.addEventListener("pointerdown", (event) => {
  const chart = event.target.closest(".chart.compare");
  if (!chart) return;
  dragFrom = compareTime(chart, event.clientX);
  dragX = event.clientX;
});

document.addEventListener("pointerup", (event) => {
  if (dragFrom === null) return;
  const chart = event.target.closest(".chart.compare");
  const from = dragFrom;
  dragFrom = null;
  // a click is not a span
  if (!chart || Math.abs(event.clientX - dragX) < 8) return;
  const to = compareTime(chart, event.clientX);
  state.recZoom = { from: Math.min(from, to), to: Math.max(from, to) };
  state.onChart = false;
  render();
});

document.addEventListener("pointermove", (event) => {
  state.onChart = Boolean(event.target.closest(".chart"));
  if (!state.onChart) return;
  if (event.target.closest(".chart.compare")) {
    showCompareCrosshair(event, t("rec.noAnswer"));
    if (dragFrom !== null) drawZoom(event);
  } else showCrosshair(event, t("rec.noAnswer"));
});
document.addEventListener("pointerout", (event) => {
  if (event.target.closest(".chart") && !event.relatedTarget?.closest?.(".chart")) {
    state.onChart = false;
    if (event.target.closest(".chart.compare")) hideCompareCrosshair();
    else hideCrosshair(event);
  }
});

window.addEventListener("hashchange", () => {
  const area = areaOfHash(location.hash);
  if (area !== state.area) enter(area);
});
window.addEventListener("beforeunload", (event) => {
  if (state.held.length) event.preventDefault();
});

/** Back to where the page was before a reload: settings, adapter, search result, open device. */
async function start() {
  const memory = loadMemory();
  Object.assign(state.settings, memory.settings ?? {});
  state.watch = loadWatch();
  state.treeView = loadTreeView();
  state.propertyNames = loadPropertyNames();
  state.area = areaOfHash(location.hash);
  installPanels();
  installColumns();
  installDragToggle();
  installPluginBridge();
  await loadPlugins();
  render();
  try {
    await loadAdapters({ ip: memory.adapterIp });
    state.scan = await api.scan();
  } catch (error) {
    state.error = error.message;
  }
  pluginsMayAsk();
  render();
  if (state.scan.state === "running") pollScan();
  const known = state.scan.devices?.some((device) => device.instance === memory.device);
  if (known) {
    const keepArea = state.area;
    await showDevice(memory.device);
    if (keepArea !== "workspace") go(keepArea);
  }
  await loadSitePoints();
  if (state.area === "watch") pollWatch();
  await Promise.all([
    loadSites(),
    refreshHeld(),
    api.log().then((log) => { state.log = log; }),
    api.recording().then((recording) => { state.recording = recording; }),
    api.info().then((info) => { state.version = info.version; }),
  ]).catch((error) => { state.error = error.message; });
  render();
  pollRecording();
  watchAdapters();
  checkForUpdate();
}

start();
