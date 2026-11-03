// Everything that needs a look across the devices read so far: faults, alarms, overrides, objects
// out of service, values you hold, devices that do not answer. A click opens the point.

import { DEVICE_TYPE, chips, deviceName, has, isFault, isHeldEntry, isOverridden, pointKey, valueHtml } from "./format.js";
import { h } from "./html.js";
import { currentLocale, objectText, t } from "./i18n.js";
import { sortHead, sortRows } from "./sorting.js";
import { filterTable } from "./table-filter.js";

function severity(point, held) {
  if (isFault(point)) return 0;
  if (isOverridden(point, held)) return 1;
  return 2;
}

/** The problems on the site, the worst first. */
export function siteProblems(state) {
  const rows = [];
  for (const device of state.scan.devices ?? []) {
    if (device.objects?.error) rows.push({ device, severity: 0, offline: device.objects.error });
    for (const point of state.sitePoints[device.instance]?.points ?? []) {
      if (point.type === DEVICE_TYPE) continue;
      const held = state.held.some((entry) => isHeldEntry(entry, device.instance, point));
      if (!isFault(point) && !isOverridden(point, held) && !has(point, "out of service")) continue;
      rows.push({ device, point, held, severity: severity(point, held) });
    }
  }
  return rows.sort((first, second) => first.severity - second.severity);
}

const PROBLEM_COLUMNS = {
  device: ({ device }) => deviceName(device),
  name: ({ point, offline }) => (point ? point.name || objectText(point) : offline),
  value: ({ point }) => point?.value ?? null,
  state: ({ severity }) => severity,
};

/** Column heads of the problems and the watch list: the same table, sorted by a click. `extra`
 * stands over a column only the watch list has. */
export const problemHead = (list, sort, extra = "") => `<div class="row problem head${extra ? " watch" : ""}" role="row">${sortHead(t("col.device"), list, "device", sort)}${sortHead(t("col.name"), list, "name", sort)}${sortHead(t("col.value"), list, "value", sort, "num")}${sortHead(t("col.state"), list, "state", sort)}${extra}</div>`;

export const unreadDevices = (state) => (state.scan.devices ?? []).filter((device) => !state.sitePoints[device.instance]);

function rowHtml({ device, point, held, offline }) {
  if (offline) {
    return `<div class="row problem" data-open-device="${device.instance}"><span class="cut sub">${h(deviceName(device))}</span><span class="name cut">${h(offline)}</span><span></span><span class="chips"><span class="chip err">${t("chip.offline")}</span></span></div>`;
  }
  return `<div class="row problem" data-open-device="${device.instance}" data-open-point="${pointKey(point)}"><span class="cut sub">${h(deviceName(device))}</span><span class="name cut">${h(point.name || objectText(point))}</span><span class="value mono">${valueHtml(point, point.state_texts)}</span><span class="chips">${chips(point, held)}</span></div>`;
}

/** What devices reported by themselves, the latest first. */
function eventsHtml(state) {
  if (!state.events.length) return "";
  const rows = [...state.events].reverse().map((event) => {
    const device = (state.scan.devices ?? []).find((found) => found.instance === event.device);
    const when = new Date(event.time * 1000).toLocaleTimeString(currentLocale());
    return `<div class="row problem" data-open-device="${event.device}" data-open-point="${event.type}:${event.instance}"><span class="name">${h(device ? deviceName(device) : t("device", { n: event.device }))}</span>
      <span class="cut">${h(event.text || objectText(event))}</span><span class="mono num">${h(when)}</span><span class="chips"><span class="chip ${event.to === "normal" ? "ok" : event.to === "fault" ? "err" : "warn"}">${h(event.to)}</span></span></div>`;
  }).join("");
  return `<div class="row group plain"><span class="name">${t("events.title")}</span><span class="count">${state.events.length}</span></div>${rows}`;
}

export function viewProblems(state) {
  const problems = siteProblems(state);
  const unread = unreadDevices(state);
  const devices = state.scan.devices ?? [];
  const note = unread.length ? `<div class="strips"><div class="strip info">${t("problems.unread", { n: unread.length, total: devices.length })}</div></div>` : "";
  const describe = ({ device, point, held, offline }) => ({ deviceInstance: device.instance, deviceName: deviceName(device), point, held, offline: offline ?? "" });
  const { shown, bar } = filterTable("problems", state.problemFilter, problems, describe);
  const rows = shown.length ? sortRows(shown, state.problemSort, PROBLEM_COLUMNS).map(rowHtml).join("") : `<div class="empty"><p>${t("table.none")}</p></div>`;
  const body = problems.length ? problemHead("problems", state.problemSort) + rows : `<div class="empty"><p>${t(devices.length ? "problems.none" : "ov.notSearched")}</p></div>`;
  return {
    title: t("tab.problems"),
    lead: t("problems.lead"),
    next: unread.length && !state.reading ? { act: "read-all", label: t("problems.readAll") } : null,
    html: `<div class="split single"><section class="listwrap">${problems.length ? bar : ""}${note}<div class="rows" data-keep="problems">${body}${eventsHtml(state)}</div></section></div>`,
  };
}
