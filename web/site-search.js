// The search over the whole site, from the sidebar: devices by name, ID and address, points of the
// devices read so far by name, description and object. A click opens what was found.

import { filterDevices } from "./devices.js";
import { DEVICE_TYPE, deviceName, pointKey } from "./format.js";
import { typeShort } from "./grouping.js";
import { h } from "./html.js";
import { objectText, t } from "./i18n.js";

/** Most rows on the page: the search is for finding one thing, not for listing a whole site. */
const MOST_RESULTS = 200;

export function searchSite(state) {
  const words = state.siteSearch.toLowerCase().split(/\s+/).filter(Boolean);
  const devices = filterDevices(state.scan.devices ?? [], state.siteSearch);
  const points = [];
  for (const device of state.scan.devices ?? []) {
    for (const point of state.sitePoints[device.instance]?.points ?? []) {
      if (point.type === DEVICE_TYPE) continue;
      const text = [point.name, point.description, objectText(point), typeShort(point.type)].join(" ").toLowerCase();
      if (words.every((word) => text.includes(word))) points.push({ device, point });
    }
  }
  return { devices, points };
}

export function viewSearch(state) {
  const { devices, points } = searchSite(state);
  const count = devices.length + points.length;
  const deviceRows = devices.map((device) => `<div class="row problem" data-open-device="${device.instance}"><span class="name">${h(deviceName(device))}</span><span class="cut sub">${t("device", { n: device.instance })}</span><span class="mono num">${h(device.address)}</span><span></span></div>`);
  const pointRows = points.map(({ device, point }) => `<div class="row problem" data-open-device="${device.instance}" data-open-point="${pointKey(point)}"><span class="name">${h(point.name || objectText(point))}</span><span class="cut sub">${h(deviceName(device))} · ${h(objectText(point))}</span><span class="mono num">${h(point.value ?? "")}</span><span></span></div>`);
  const asked = (state.whoHas?.found ?? []).map((found) => `<div class="row problem" data-open-device="${found.device}" data-open-point="${found.type}:${found.instance}"><span class="name">${h(found.name)}</span><span class="cut sub">${t("device", { n: found.device })} · ${h(objectText(found))}</span><span></span><span></span></div>`);
  const rows = [...deviceRows, ...pointRows, ...asked].slice(0, MOST_RESULTS).join("");
  const ask = `<div class="toolbar"><span class="hint">${t("whoHas.hint")}</span><div class="end"><button type="button" class="quiet" data-act="who-has"${state.whoHas?.asking ? " disabled" : ""}>${t("whoHas.ask")}</button></div></div>`;
  return {
    title: t("search.title"),
    lead: t("search.lead", { n: count }),
    next: null,
    html: `<div class="split single"><section class="listwrap">${ask}<div class="rows" data-keep="search">${rows || `<div class="empty"><p>${t("search.none")}</p></div>`}</div></section></div>`,
  };
}
