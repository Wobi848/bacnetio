// Typing and choosing in fields: what goes straight into the state, and what redraws a list.

import { importSite, loadSitePoints } from "./site-actions.js";
import { saveMemory } from "./memory.js";
import { installPlugin } from "./plugin-actions.js";
import { CHANGES } from "./change-many.js";
import { TABLE_FILTERS } from "./table-filter.js";
import { chooseScFile } from "./sc-actions.js";
import { forgetDeviceReads, render, state } from "./store.js";

/** Fields whose typing goes straight into the state. */
export const FIELDS = {
  bbmd: (value) => { state.settings.bbmd = value; },
  port: (value) => { state.settings.port = value; },
  subnet: (value) => { state.settings.subnet = value; },
  "id-from": (value) => { state.settings.idFrom = value; },
  "id-to": (value) => { state.settings.idTo = value; },
  "mstp-station": (value) => { state.mstp.station = value; },
  "locate-address": (value) => { state.settings.locate = value; },
  "locate-network": (value) => { state.settings.locateNetwork = value; },
  "locate-mac": (value) => { state.settings.locateMac = value; },
  "locate-id": (value) => { state.settings.locateId = value; },
};

/** Fields that filter a list and redraw it while typing. */
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

document.addEventListener("change", (event) => {
  if (event.target.id === "plugin-file") {
    if (event.target.files[0]) installPlugin(event.target.files[0]);
    return;
  }
  if (event.target.id.startsWith("sc-file-")) {
    chooseScFile(event.target.dataset.line, event.target.files[0]);
    return;
  }
  if (event.target.id === "sc-remember") {
    state.sc.rememberKey = event.target.checked;
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
  if (event.target.id === "mstp-port" || event.target.id === "mstp-baud") {
    state.mstp[event.target.id === "mstp-port" ? "port" : "baud"] = event.target.value;
    state.mstp.heard = null;
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
  if (id === "grouping-pattern") { state.groupingEdit = event.target.value; render(); }
  if (id === "many-kind") {
    state.changeMany.kind = event.target.value;
    state.changeMany.value = CHANGES[event.target.value].yesNo ? "true" : "";
    render();
  }
  if (id === "many-value") { state.changeMany.value = event.target.value; render(); }
  if (id === "many-priority") { state.changeMany.priority = Number(event.target.value); render(); }
  if (id.startsWith("sc-password-")) state.sc.passwords[id.slice("sc-password-".length)] = event.target.value;
  // the button to connect follows the hub typed
  if (id === "sc-hub") { state.sc.hub = event.target.value; render(); }
  if (FIELDS[id]) FIELDS[id](event.target.value);
  if (FILTERS[id]) { FILTERS[id](event.target.value); render(); }
  if (id === "table-filter") { state[TABLE_FILTERS[event.target.dataset.list]].text = event.target.value; render(); }
});
