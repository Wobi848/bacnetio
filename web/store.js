// The UI state and how it reaches the screen, shared by the action modules. Each page view returns
// { title, lead, next, html }; the shell draws the sidebar, page head and status bar around it.

import { api } from "./api.js";
import { viewLog } from "./log.js";
import { viewProblems } from "./problems.js";
import { viewRecording } from "./recording.js";
import { viewSettings } from "./settings.js";
import { viewSites } from "./sites.js";
import { viewSearch } from "./site-search.js";
import { viewWatch } from "./watch.js";
import { viewWorkspace } from "./workspace.js";
import { nextbarHtml, sideHtml, statusHtml } from "./shell.js";

/** The areas in the order of their keys 1 to 6; the sites are reached from the top of the sidebar. */
export const AREAS = ["workspace", "problems", "watch", "recording", "log", "settings", "sites"];
/** Addresses of the pages the workspace replaced, so that old links and bookmarks still lead there. */
const FORMER_AREAS = ["overview", "devices", "points"];

/** The area an address in the browser means. */
export function areaOfHash(hash) {
  const area = hash.replace("#/", "");
  if (FORMER_AREAS.includes(area)) return "workspace";
  return AREAS.includes(area) ? area : "workspace";
}


export const state = {
  area: "workspace",
  adapters: [],
  adapter: null,
  /** Search settings from the "Change" form in the sidebar. */
  settings: { bbmd: "", port: "47808", locate: "", locateNetwork: "", locateMac: "", locateId: "", idFrom: "", idTo: "", subnet: "" },
  /** Which menu is open: "network", "locate" or "export". */
  menu: null,
  scan: { state: "idle" },
  deviceFilter: "",
  /** How the lists are sorted: { key, descending }, or null for the device's own order. */
  deviceSort: null,
  pointSort: null,
  problemSort: null,
  watchSort: null,
  /** Search, view and device of the site problems and the watch list (table-filter.js). */
  problemFilter: { text: "", view: "all", device: null },
  watchFilter: { text: "", view: "all", device: null },
  /** Points set to be asked each time instead of following COV, by "device:type:instance". */
  liveModes: {},
  /** The rows on screen update by themselves. */
  liveList: false,
  /** When each device's points were last read here. */
  readAt: {},
  /** All groups open or closed by choice; groups clicked since stand apart. */
  groupsDefault: null,
  /** The flat point list shows all its rows, however many. */
  allRows: false,
  /** Chosen row in the device list, and what is known about it beyond the scan. */
  selectedDevice: null,
  deviceFacts: null,
  /** The chosen device's active alarms: null while read, a list, or { error }. */
  deviceAlarms: null,
  /** All active alarms of the device in its details, not only the first. */
  alarmsAll: false,
  /** The BBMD tables read for a device, by device ID, and the device being read right now. */
  bbmd: {},
  bbmdReading: null,
  /** The search over the whole site, from the sidebar; while it holds text, it is the page. */
  siteSearch: "",
  /** Points of the devices read, by device ID, for the problems and the search. */
  sitePoints: {},
  /** Alarm messages devices sent by themselves. */
  events: [],
  /** The tree shows a device's objects by "type" or by "plant" part. */
  treeView: "type",
  /** Devices whose types the tree shows without their being open. */
  treeOpen: {},
  /** The node of the open device the table shows: "part:…", "kind:…", "view:…", "type:…", or null for all. */
  node: null,
  /** The plant view a device carries itself (Structured View objects), by device ID. */
  structure: {},
  /** Sites kept in the data folder, the one whose removal is being asked about, and the one being
   * renamed with the name typed for it. */
  sites: [],
  deletingSite: null,
  renamingSite: null,
  siteTitle: "",
  /** The priority that wins on each object of the open device, once asked for. */
  winners: {},
  readingWinners: false,
  /** The property being changed in the technical details, and what is typed for it. */
  editProperty: null,
  editValue: "",
  /** Properties whose value changed while the technical details were open. */
  changedProperties: {},
  /** Points to keep an eye on across devices: { device, type, instance, name, value, status }. */
  watch: [],
  /** The device whose points are open. */
  device: null,
  points: null,
  pointNotes: [],
  reading: false,
  /** The background reading's state while it runs: phase, counts. */
  readProgress: null,
  /** The chosen device's request log is shown, and its entries. */
  requestLog: false,
  requests: [],
  filter: "",
  /** "all", "faults", "overridden", "alarm", "outOfService", "held" or "changed". */
  pointView: "all",
  /** The object types chosen, as numbers; none chosen shows all. */
  pointTypes: [],
  /** Groups the user opened or closed, by key; others follow the default. */
  groupsOpen: {},
  /** The open device against its saved reference. */
  reference: null,
  /** Object names by "device:type:instance", to name held values. */
  objectNames: {},
  /** Chosen point and its details read from the device. */
  selectedPoint: null,
  object: null,
  objectError: null,
  /** The details are shown over the list (phone). */
  detailOpen: false,
  writeUnlocked: false,
  // the device being read now, and whether someone asked to stop
  readingDevice: null,
  stopping: false,
  /** What is typed or chosen for the next write, so that a redraw does not take it away. */
  newValue: null,
  writePriority: 8,
  /** A write or release that would not act, waiting for "anyway". */
  confirm: null,
  /** The device's commands are shown in its details. */
  commandsOpen: false,
  /** What is typed for a command to the device; the password is never kept beyond it. */
  commandPassword: "",
  commandMinutes: "",
  /** What is typed for the device's new ID. */
  newInstance: "",
  newName: "",
  /** The question before leaving the site with values held. */
  leaving: false,
  asking: false,
  technical: false,
  properties: null,
  /** The properties in plain words or by their BACnet names (properties.js). */
  propertyNames: "plain",
  propertyFilter: "",
  /** The properties outside the sections are shown. */
  propertiesOther: false,
  liveSource: null,
  trend: null,
  /** A schedule's week or a calendar's dates: null while read, or { error }. */
  times: null,
  /** The day of the schedule being changed, 0 is Monday, and what is typed for it. */
  editDay: null,
  editDayText: "",
  /** Answers to the question who has an object of a name; null before it was asked. */
  whoHas: null,
  held: [],
  log: [],
  recording: null,
  recordInterval: 10,
  /** How the recording is looked at: "compare" or "cards"; the last minutes, or a span drawn with the pointer. */
  recView: "compare",
  recWindow: null,
  recZoom: null,
  /** The colour of each recorded point, kept for as long as the page is open. */
  recColors: {},
  version: null,
  /** What the last look for a new version found (update.js). */
  update: null,
  /** The pointer is on a chart; a redraw would drop the crosshair. */
  onChart: false,
  error: null,
};

const VIEWS = { workspace: viewWorkspace, problems: viewProblems, watch: viewWatch, recording: viewRecording, log: viewLog, settings: viewSettings, sites: viewSites };

/** Scroll positions and the focused field survive a redraw: the working context stays. */
function remember() {
  const scroll = [...document.querySelectorAll("[data-keep]")].map((element) => [element.dataset.keep, element.scrollTop]);
  const active = document.activeElement;
  const focus = active?.id ? { id: active.id, start: active.selectionStart, end: active.selectionEnd } : null;
  return { scroll, focus };
}

function restore({ scroll, focus }) {
  for (const [key, top] of scroll) {
    const element = document.querySelector(`[data-keep="${key}"]`);
    if (element) element.scrollTop = top;
  }
  const field = focus && document.getElementById(focus.id);
  if (!field) return;
  field.focus();
  if (focus.start !== null && typeof field.setSelectionRange === "function") {
    try {
      field.setSelectionRange(focus.start, focus.end);
    } catch {
      // fields like number inputs have no caret to put back
    }
  }
}

export function render() {
  const kept = remember();
  const view = state.siteSearch.trim() ? viewSearch(state) : VIEWS[state.area](state);
  document.getElementById("side").innerHTML = sideHtml(state);
  document.getElementById("nextbar").innerHTML = nextbarHtml(view);
  document.getElementById("work").innerHTML = view.html;
  document.getElementById("status").innerHTML = statusHtml(state);
  document.title = `${view.title} · bacnetio`;
  restore(kept);
}

export function go(area) {
  state.area = area;
  state.menu = null;
  state.error = null;
  state.detailOpen = false;
  state.siteSearch = "";
  if (location.hash !== `#/${area}`) history.replaceState(null, "", `#/${area}`);
  render();
  if (area === "log") api.log().then((log) => { state.log = log; render(); }, (error) => toast(error.message, true));
}

let toastTimer;
export function toast(text, error = false) {
  const element = document.getElementById("toast");
  element.textContent = text;
  element.classList.toggle("error", error);
  element.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { element.hidden = true; }, 3500);
}

export async function refreshHeld() {
  state.held = await api.held();
}

/**
 * What was read about the devices themselves and kept by device ID: after a new search or another
 * site the same ID can be another device, so it must be read again.
 */
export function forgetDeviceReads() {
  state.structure = {};
  state.bbmd = {};
}
