// The workspace: the tree of the site, the table of what is chosen in it, and the details beside.
// With no device open, the table is the device list; with one open, its points.

import { viewDevices } from "./devices.js";
import { t } from "./i18n.js";
import { releaseLabel } from "./log.js";
import { viewPoints } from "./points.js";
import { viewPrint } from "./print.js";
import { treeHtml } from "./tree.js";

/** The one next step of the page head. */
export function nextStep(state) {
  if (state.held.length) {
    return { act: "go", data: ' data-area="log"', label: releaseLabel(state.held.length) };
  }
  if (state.scan.state === "running") return null;
  if (state.scan.remembered_ms) return searchStep(state);
  if (state.device) return null;
  if (state.selectedDevice !== null && state.scan.devices?.some((device) => device.instance === state.selectedDevice)) {
    return { act: "open-device", data: ` data-id="${state.selectedDevice}"`, label: t("dev.openPoints") };
  }
  return searchStep(state);
}

/** A search, named "Search again" once there is a result, kept from before a restart or not. */
function searchStep(state) {
  const searched = state.scan.state !== "idle" || state.scan.remembered_ms;
  return { act: "find", label: searched ? t("searchAgain") : t("find") };
}

/** While values are held, releasing them is the next step, but a new search stays at hand beside it. */
function searchBeside(state) {
  if (!state.held.length || state.scan.state === "running") return null;
  return searchStep(state);
}

export function viewWorkspace(state) {
  if (state.printing && state.device && state.points) return viewPrint(state);
  const also = searchBeside(state);
  const inner = state.device ? viewPoints(state, also) : viewDevices(state, also);
  return { ...inner, next: nextStep(state), also, html: `<div class="workspace">${treeHtml(state)}${inner.html}</div>` };
}
