// Links to the Excel files (docs/concept-excel.md): the server builds them from the same rows as
// the CSV, and the page gives the column heads and a title line in the reader's language. The CSV
// keeps its English heads for scripts.

import { currentLocale, t } from "./i18n.js";
import { openSiteOf, siteName } from "./sites.js";

/** The English heads and sheet names of src/export.rs, each with its text key. */
export const HEADS = {
  Device: "xlsx.device", "Object type": "xlsx.objectType", Instance: "xlsx.instance", Name: "xlsx.name",
  Description: "xlsx.description", Value: "xlsx.value", Unit: "xlsx.unit", Status: "xlsx.status",
  Error: "xlsx.error", "Winning priority": "xlsx.winningPriority", "Set priorities": "xlsx.setPriorities",
  "Device ID": "xlsx.deviceId", Vendor: "xlsx.vendor", Model: "xlsx.model", Address: "xlsx.address",
  Network: "xlsx.network", MAC: "xlsx.mac", "Found by": "xlsx.foundBy", Time: "xlsx.time",
  Priority: "xlsx.priority", Action: "xlsx.action", Refused: "xlsx.refused",
  Devices: "xlsx.devices", Log: "xlsx.log", Recording: "xlsx.recording",
};

/** The line above the header: the site, what is exported, when, and with which bacnetio. */
function title(state, what) {
  const site = openSiteOf(state);
  const made = new Date().toLocaleString(currentLocale());
  return [site ? siteName(site) ?? site.name : "", ...what, made, `bacnetio ${state.version ?? ""}`.trim()].filter(Boolean).join(" · ");
}

/** The address of an Excel file: `path` with the heads, the title and the viewer's time zone. */
export function excelHref(state, path, what = []) {
  const heads = Object.fromEntries(Object.entries(HEADS).map(([english, key]) => [english, t(key)]));
  const query = new URLSearchParams({
    heads: JSON.stringify(heads),
    title: title(state, what),
    offset_min: String(-new Date().getTimezoneOffset()),
  });
  return `${path}?${query}`;
}

/** An Excel link as the CSV links are made, with the same words and "Excel" after them. */
export const excelLink = (state, path, label, what = []) =>
  `<a class="quiet" href="${excelHref(state, path, what).replaceAll("&", "&amp;")}" download>${label}</a>`;
