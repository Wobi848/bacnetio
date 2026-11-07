// Talks to bacnetio's JSON API (docs/api.md). Errors come back as Error with the server's text.

import { errorText, t } from "./i18n.js";

async function call(method, path, body, contentType = "application/json") {
  const options = { method, headers: {} };
  if (body !== undefined) {
    options.headers["content-type"] = contentType;
    options.body = contentType === "application/json" ? JSON.stringify(body) : body;
  }
  let response;
  try {
    response = await fetch(path, options);
  } catch {
    throw new Error(t("notRunning"));
  }
  const text = await response.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    // not an answer of this bacnetio's API: a proxy, or a program older than this page
    throw new Error(t("badAnswer", { path }));
  }
  if (!response.ok) {
    const error = new Error(data?.error ? errorText(data.error) : `${response.status} ${response.statusText}`);
    error.status = response.status;
    throw error;
  }
  return data;
}

const objectPath = (device, type, instance) => `/api/devices/${device}/objects/${type}/${instance}`;

export const api = {
  info: () => call("GET", "/api/info"),
  plugins: () => call("GET", "/api/plugins"),
  installPlugin: (page, replace) => call("POST", `/api/plugins${replace ? "?replace=true" : ""}`, page, "text/html"),
  switchPlugin: (id, on) => call("POST", `/api/plugins/${encodeURIComponent(id)}/switch`, { on }),
  removePlugin: (id) => call("POST", `/api/plugins/${encodeURIComponent(id)}/remove`),
  pluginStore: (id) => call("GET", `/api/plugins/${encodeURIComponent(id)}/store`),
  savePluginStore: (id, data) => call("POST", `/api/plugins/${encodeURIComponent(id)}/store`, data),
  update: () => call("GET", "/api/update"),
  adapters: () => call("GET", "/api/adapters"),
  serialPorts: () => call("GET", "/api/serial-ports"),
  mstpListen: (port, baud) => call("POST", "/api/mstp/listen", baud ? { port, baud } : { port }),
  startScan: (settings) => call("POST", "/api/scan", settings),
  stopScan: () => call("POST", "/api/scan/stop"),
  locate: (request) => call("POST", "/api/devices/locate", request),
  scan: () => call("GET", "/api/scan"),
  sitePoints: () => call("GET", "/api/site/points"),
  sites: () => call("GET", "/api/sites"),
  openSite: (name) => call("POST", "/api/sites/open", { name }),
  deleteSite: (name) => call("POST", "/api/sites/delete", { name }),
  renameSite: (name, title) => call("POST", "/api/sites/rename", { name, title }),
  importSite: (site) => call("POST", "/api/site.json", site),
  changeInstance: (device, instance) => call("POST", `/api/devices/${device}/instance`, { instance }),
  restart: (device, kind, password) => call("POST", `/api/devices/${device}/restart`, { kind, password }),
  communication: (device, enable, minutes, password) => call("POST", `/api/devices/${device}/communication`, { enable, minutes, password }),
  acknowledge: (device, request) => call("POST", `/api/devices/${device}/alarms/acknowledge`, request),
  schedule: (device, type, instance) => call("GET", `${objectPath(device, type, instance)}/schedule`),
  writeDay: (device, type, instance, day, changes) => call("POST", `${objectPath(device, type, instance)}/schedule`, { day, changes }),
  calendar: (device, type, instance) => call("GET", `${objectPath(device, type, instance)}/calendar`),
  events: () => call("GET", "/api/events"),
  whoHas: (name) => call("POST", "/api/who-has", { name }),
  liveValues: (device, objects) => call("POST", `/api/devices/${device}/values`, { objects }),
  priorities: (device) => call("GET", `/api/devices/${device}/priorities`),
  structure: (device) => call("GET", `/api/devices/${device}/structure`),
  bbmdTables: (device) => call("GET", `/api/devices/${device}/bbmd`),
  writeProperty: (device, type, instance, property) => call("POST", `${objectPath(device, type, instance)}/property`, property),
  points: (device) => call("GET", `/api/devices/${device}/points`),
  lastPoints: (device) => call("GET", `/api/devices/${device}/points/last`),
  startRead: (device) => call("POST", `/api/devices/${device}/read`),
  stopRead: (device) => call("POST", `/api/devices/${device}/read/stop`),
  alarms: (device) => call("GET", `/api/devices/${device}/alarms`),
  readProgress: (device) => call("GET", `/api/devices/${device}/read`),
  setClock: (device, clock) => call("POST", `/api/devices/${device}/clock`, clock),
  reference: (device) => call("GET", `/api/devices/${device}/reference`),
  saveReference: (device) => call("POST", `/api/devices/${device}/reference`),
  object: (device, type, instance) => call("GET", objectPath(device, type, instance)),
  trend: (device, type, instance) => call("GET", `${objectPath(device, type, instance)}/trend`),
  value: (device, type, instance, mode) => call("GET", `${objectPath(device, type, instance)}/value${mode === "read" ? "?mode=read" : ""}`),
  properties: (device, type, instance) => call("GET", `${objectPath(device, type, instance)}/properties`),
  write: (device, type, instance, value, priority) =>
    call("POST", `${objectPath(device, type, instance)}/write`, { value, priority }),
  release: (device, type, instance, priority, onlyMine = false) =>
    call("POST", `${objectPath(device, type, instance)}/release`, { priority, only_mine: onlyMine }),
  held: () => call("GET", "/api/held"),
  log: () => call("GET", "/api/log"),
  recording: () => call("GET", "/api/recording"),
  recordAdd: (device, type, instance) => call("POST", "/api/recording/points", { device, type, instance }),
  recordRemove: (device, type, instance) => call("POST", "/api/recording/points/remove", { device, type, instance }),
  recordClear: () => call("POST", "/api/recording/clear"),
  recordMode: (device, type, instance, mode) => call("POST", "/api/recording/mode", { device, type, instance, mode }),
  recordStart: (seconds) => call("POST", "/api/recording/start", { interval_s: seconds }),
  recordStop: () => call("POST", "/api/recording/stop"),
};
