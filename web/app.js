// Starts the page and hands clicks to the actions (actions.js). The areas are addressed by the
// hash (#/devices), so the browser's back button and a reload keep the place.

import { loadAdapters, watchAdapters } from "./adapter-actions.js";
import { api } from "./api.js";
import { pollScan } from "./device-actions.js";
import { loadSitePoints, loadSites, renameSite } from "./site-actions.js";
import { loadMemory, loadPropertyNames, loadTreeView, loadWatch } from "./memory.js";
import { installColumns } from "./columns.js";
import { installPanels } from "./panels.js";
import { installDragToggle } from "./drag-toggle.js";
import { selectPoint } from "./object-actions.js";
import { installPluginBridge, loadPlugins, pluginsMayAsk } from "./plugin-actions.js";
import { pollRecording } from "./recording-actions.js";
import { loadTrunk } from "./mstp-actions.js";
import { loadCapture } from "./capture-actions.js";
import { checkForUpdate } from "./update.js";
import { areaOfHash, go, refreshHeld, render, state } from "./store.js";
import { pollWatch } from "./watch-actions.js";
import { enter, openFound, showDevice } from "./navigation.js";
import { actions } from "./actions.js";
import "./fields.js";
import "./keys.js";
import "./chart-pointer.js";

function toggleGroup(row) {
  state.groupsOpen[row.dataset.group] = row.getAttribute("aria-expanded") !== "true";
  render();
}

document.addEventListener("click", (event) => {
  const target = event.target;
  if (state.menu && !target.closest(".menu, .modal-box, [data-act=network], [data-act=site-menu], [data-act=locate-menu], [data-act=export-menu], [data-act=more-menu], [data-act=types-menu]")) {
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
    api.info().then((info) => {
      state.version = info.version;
      state.mstpAvailable = Boolean(info.mstp);
      if (state.mstpAvailable) return loadTrunk();
    }),
  ]).catch((error) => { state.error = error.message; });
  render();
  pollRecording();
  loadCapture();
  watchAdapters();
  checkForUpdate();
}

start();

// a chart is drawn for its width: a wider or narrower window draws the page again
let resizing = 0;
window.addEventListener("resize", () => {
  cancelAnimationFrame(resizing);
  resizing = requestAnimationFrame(() => render());
});
