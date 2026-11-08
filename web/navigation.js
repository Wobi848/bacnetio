// Moving between the areas, the site and its devices, and closing what lies on top.

import { openDevice, openPointAt } from "./device-actions.js";
import { loadSitePoints, loadSites } from "./site-actions.js";
import { t } from "./i18n.js";
import { saveMemory } from "./memory.js";
import { go, render, state, toast } from "./store.js";
import { loadStructure } from "./structure-actions.js";
import { pollWatch } from "./watch-actions.js";

/** Opens the name field of a kept site, with its name so far. */
export function startRename(index) {
  state.renamingSite = index;
  state.siteTitle = state.sites[index].title ?? "";
  render();
  document.getElementById("site-title")?.focus();
}

/** Goes to an area, with what that area needs loaded. */
export function enter(area) {
  go(area);
  if (area === "problems") loadSitePoints();
  if (area === "watch") pollWatch();
  if (area === "sites") loadSites();
}

/** Back to the site as a whole: the device list in the table. */
export function showSite() {
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
export async function showDevice(instance, fresh = false) {
  await openDevice(instance, fresh);
  await loadStructure(instance, fresh);
}

export async function chooseNode(element) {
  const device = Number(element.dataset.device);
  if (state.device?.instance !== device) await showDevice(device);
  state.node = element.dataset.node;
  render();
}

export function leaveSite() {
  showSite();
  toast(t("leave.done"));
}

export async function openFound(row) {
  state.siteSearch = "";
  const device = Number(row.dataset.openDevice);
  await openPointAt(device, row.dataset.openPoint);
  await loadStructure(device);
}

/** The open device's points as a page to print, with the list's groups, filter and view. */
export function openPrint() {
  state.printing = true;
  state.menu = null;
  render();
}

/** Closes the menu, dialog, search or details on top; false when nothing was open. */
export function closeTopmost() {
  if (state.menu) state.menu = null;
  else if (state.pluginRemoving || state.pluginReplacing) { state.pluginRemoving = null; state.pluginReplacing = null; }
  else if (state.leaving) state.leaving = false;
  else if (state.renamingSite !== null) state.renamingSite = null;
  else if (state.siteSearch || document.activeElement?.id === "site-search") {
    state.siteSearch = "";
    document.activeElement.blur();
  }
  else if (state.confirm) state.confirm = null;
  else if (state.asking) state.asking = false;
  else if (state.detailOpen) state.detailOpen = false;
  else if (state.groupingEdit !== null) state.groupingEdit = null;
  else if (state.compare) state.compare = null;
  else if (state.changeMany && state.changeMany.phase !== "writing") state.changeMany = null;
  else if (state.printing) state.printing = false;
  else return false;
  return true;
}

/** The button each menu or dialog opens from: the focus goes back to it when it closes. */
const OPENERS = { sites: "site-menu", network: "network", export: "export-menu", more: "more-menu", types: "types-menu", locate: "locate-menu" };

/** Closes the open menu or dialog and gives the focus back to the button that opened it. */
export function closeMenu() {
  const opener = OPENERS[state.menu];
  state.menu = null;
  render();
  if (opener) document.querySelector(`[data-act="${opener}"]`)?.focus();
}
