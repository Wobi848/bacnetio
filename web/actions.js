// What each button does: its data-act names the entry.

import { api } from "./api.js";
import { readBbmdTables } from "./bbmd-actions.js";
import { acknowledgeAlarm, askAcknowledge, askWhoHas, changeInstance, readWinners, renameDevice, runCommand } from "./device-commands.js";
import { find, locate, saveReference, toggleRequests } from "./device-actions.js";
import { deleteSite, exportSiteEde, exportSiteExcel, groupSite, loadSites, openSite, readAllDevices, stopRunning } from "./site-actions.js";
import { setLanguage, t } from "./i18n.js";
import { toggleLiveList } from "./live-list.js";
import { savePropertyNames, saveTreeView } from "./memory.js";
import { editDay, editProperty, releaseHeld, releaseHere, releaseOther, setClock, showTechnical, writeDay, writeProperty, writeValue } from "./object-actions.js";
import { installPlugin, removePlugin, switchPlugin } from "./plugin-actions.js";
import { recordHere, recordingRequest, recordPoint, removeRecorded, setRecordMode } from "./recording-actions.js";
import { chooseLink, leaveTrunk, listenToTrunk, loadSerialPorts, searchOnTrunk, searchTrunkAgain } from "./mstp-actions.js";
import { setAccent, setBackground, setTheme } from "./settings.js";
import { openSiteOf, siteName } from "./sites.js";
import { chooseCompare, openComparedPoint, readBoth, startCompare } from "./compare-actions.js";
import { captureRequest } from "./capture-actions.js";
import { openChangeMany, pickGroup, pickNone, pickShown, togglePicked, toggleSelecting, writeMany } from "./change-many-actions.js";
import { nextSort } from "./sorting.js";
import { checkForUpdate, setUpdateCheck } from "./update.js";
import { noTableFilter, TABLE_FILTERS } from "./table-filter.js";
import { forgetDeviceReads, go, render, state, toast } from "./store.js";
import { unwatch, unwatchHere, unwatchPoint, watchHere, watchPoint } from "./watch-actions.js";
import { isRecorded, isWatched, pointKey } from "./format.js";
import { chooseNode, closeMenu, enter, leaveSite, openPrint, showDevice, showSite, startRename } from "./navigation.js";

/** Which sort each table keeps. */
const SORTS = { devices: "deviceSort", points: "pointSort", problems: "problemSort", watch: "watchSort" };

/** The chosen device's name as last read, to start a new one from. */
const deviceNameNow = () => (state.scan.devices ?? []).find((device) => device.instance === state.selectedDevice)?.name ?? "";

const listedPoint = (key) => state.points.find((point) => pointKey(point) === key);

export const actions = {
  // with MS/TP chosen, searching again searches the trunk bacnetio is on
  find: () => { forgetDeviceReads(); if (state.link === "mstp" && state.mstp.trunk) searchTrunkAgain(); else find(); },
  locate: () => locate(),
  network: () => { state.menu = state.menu === "network" ? null : "network"; render(); document.getElementById("subnet")?.focus(); },
  "locate-menu": () => { state.menu = state.menu === "locate" ? null : "locate"; render(); document.getElementById("locate-address")?.focus(); },
  "close-menu": () => closeMenu(),
  link: (element) => chooseLink(element.dataset.link),
  "mstp-ports": () => loadSerialPorts(),
  "mstp-listen": () => listenToTrunk(),
  "mstp-search": () => { forgetDeviceReads(); searchOnTrunk(); },
  "mstp-leave": () => {
    if (state.held.length) { state.mstp.leaving = true; render(); } else leaveTrunk();
  },
  "mstp-leave-anyway": () => leaveTrunk(),
  "toggle-head": () => { state.headOpen = !state.headOpen; render(); },
  // the adapter is chosen in the search settings: open them with the choice in focus
  "focus-adapter": () => {
    state.menu = "network";
    render();
    document.getElementById("adapter")?.focus();
  },
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
  "grouping-edit": () => {
    state.groupingEdit = openSiteOf(state)?.grouping ?? "";
    state.menu = null;
    render();
    document.getElementById("grouping-pattern")?.focus();
  },
  // spaces count in a pattern: only one of nothing else is none
  "grouping-save": () => groupSite(openSiteOf(state).name, state.groupingEdit.trim() ? state.groupingEdit : null),
  "grouping-auto": () => groupSite(openSiteOf(state).name, null),
  "grouping-cancel": () => { state.groupingEdit = null; render(); },
  "compare-start": () => startCompare(),
  "compare-choose": (element) => chooseCompare(Number(element.dataset.device)),
  "compare-view": (element) => { state.compare.view = element.dataset.view; render(); },
  "compare-read": () => readBoth(),
  "compare-close": () => { state.compare = null; render(); },
  "compare-open": (element) => openComparedPoint(Number(element.dataset.device), element.dataset.key),
  "select-toggle": () => toggleSelecting(),
  "pick-point": (element) => togglePicked(element.dataset.key),
  "pick-shown": () => pickShown(),
  "pick-group": (element) => pickGroup(element.dataset.group),
  "pick-none": () => pickNone(),
  "many-open": () => openChangeMany(),
  "many-ask": () => { state.changeMany.phase = "confirm"; render(); },
  "many-back": () => { state.changeMany.phase = "edit"; render(); },
  "many-write": () => writeMany(),
  "many-stop": () => { state.changeMany.stopping = true; render(); },
  "many-close": () => { state.changeMany = null; render(); },
  "capture-start": () => captureRequest("start"),
  "capture-stop": () => captureRequest("stop"),
  "capture-clear": () => captureRequest("clear"),
  "print-open": () => openPrint(),
  "print-now": () => window.print(),
  "print-close": () => { state.printing = false; render(); },
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
  "site-open": (element) => { state.menu = null; forgetDeviceReads(); openSite(state.sites[Number(element.dataset.index)].name); },
  "site-ask-delete": (element) => { state.deletingSite = Number(element.dataset.index); render(); },
  "site-delete": () => deleteSite(state.sites[state.deletingSite].name),
  "site-keep": () => { state.deletingSite = null; render(); },
  "site-rename": (element) => startRename(Number(element.dataset.index)),
  // the site field: the known sites to switch to, fetched again while it opens
  "site-menu": () => {
    state.menu = state.menu === "sites" ? null : "sites";
    render();
    document.querySelector(".site-choices [aria-checked=true], .site-choices button")?.focus();
    if (state.menu === "sites") loadSites();
  },
  // the page of all sites; a site without a name has its name field ready to type there
  "sites-all": async () => {
    state.menu = null;
    go("sites");
    await loadSites();
    const index = state.sites.findIndex((site) => site.open && !siteName(site));
    if (index >= 0) startRename(index);
  },
  "site-rename-cancel": () => { state.renamingSite = null; render(); },
  // a new site starts with a search; the network form says where
  "site-new": () => { enter("workspace"); actions.network(); },
  // on a phone the guide stands below the list: it comes into view when chosen
  "help-guide": (element) => {
    state.helpGuide = element.dataset.guide;
    if (state.area === "help") render();
    else enter("help");
    document.getElementById("help-guide")?.scrollIntoView({ block: "nearest" });
  },
  // a guide only opens the place of its first step, it never acts
  "help-go": (element) => {
    const target = element.dataset.target;
    if (target === "network") actions["site-new"]();
    else if (target === "sites") actions["sites-all"]();
    else enter(target);
  },
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
  "site-excel": () => exportSiteExcel(),
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
  "update-details": () => { state.updateDetails = !state.updateDetails; render(); },
  "plugin-switch": (element) => switchPlugin(element.dataset.id, element.dataset.on === "true"),
  "plugin-remove": (element) => { state.pluginRemoving = element.dataset.id; render(); document.querySelector(".modal-box button")?.focus(); },
  "plugin-remove-yes": (element) => removePlugin(element.dataset.id),
  "plugin-remove-no": () => { state.pluginRemoving = null; render(); },
  "plugin-replace-yes": () => installPlugin(state.pluginReplacing.page, true),
  "plugin-replace-no": () => { state.pluginReplacing = null; render(); },
  accent: (element) => { setAccent(element.dataset.accent); render(); },
  background: (element) => { setBackground(element.dataset.scheme, element.dataset.background); render(); },
};
