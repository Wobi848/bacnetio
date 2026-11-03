// The workspace: the tree of the site, the table of what is chosen in it, and the details beside.
// With no device open, the table is the device list; with one open, its points.

import { viewDevices } from "./devices.js";
import { t } from "./i18n.js";
import { viewPoints } from "./points.js";
import { treeHtml } from "./tree.js";

/** The one next step of the page head. */
export function nextStep(state) {
  if (state.held.length) {
    return { act: "go", data: ' data-area="log"', label: state.held.length === 1 ? t("next.releaseOne") : t("next.release", { n: state.held.length }) };
  }
  if (state.scan.state === "running") return null;
  if (state.scan.remembered_ms) return { act: "find", label: t("searchAgain") };
  if (state.device) return null;
  if (state.selectedDevice !== null && state.scan.devices?.some((device) => device.instance === state.selectedDevice)) {
    return { act: "open-device", data: ` data-id="${state.selectedDevice}"`, label: t("dev.openPoints") };
  }
  return { act: "find", label: state.scan.state === "idle" ? t("find") : t("searchAgain") };
}

export function viewWorkspace(state) {
  const inner = state.device ? viewPoints(state) : viewDevices(state);
  return { ...inner, next: nextStep(state), html: `<div class="workspace">${treeHtml(state)}${inner.html}</div>` };
}
