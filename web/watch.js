// The watch list: points you keep an eye on across devices, with their values live.

import { chips, deviceName, isHeldEntry, pointKey } from "./format.js";
import { h } from "./html.js";
import { objectText, t, valueWord } from "./i18n.js";
import { problemHead } from "./problems.js";
import { sortRows } from "./sorting.js";
import { filterTable } from "./table-filter.js";

const deviceLabel = (state, entry) => {
  const device = state.scan.devices?.find((found) => found.instance === entry.device);
  return device ? deviceName(device) : t("device", { n: entry.device });
};

/** `index` is the entry's place in the watch list, which a sorted table does not show. */
function rowHtml(state, { entry, index }) {
  const held = state.held.some((item) => isHeldEntry(item, entry.device, entry));
  const value = entry.value === null || entry.value === undefined ? `<span class="sub">${t("watch.waiting")}</span>` : h(valueWord(entry.value));
  return `<div class="row problem watch" data-open-device="${entry.device}" data-open-point="${pointKey(entry)}"><span class="cut sub">${h(deviceLabel(state, entry))}</span>
    <span class="name cut">${h(entry.name || objectText(entry))}</span><span class="mono num value">${value}</span>
    <span class="chips">${chips(entry, held)}</span><button type="button" class="quiet" data-act="watch-remove" data-index="${index}">${t("watch.removeShort")}</button></div>`;
}

const describe = (state) => ({ entry }) => ({
  deviceInstance: entry.device,
  deviceName: deviceLabel(state, entry),
  point: entry,
  held: state.held.some((item) => isHeldEntry(item, entry.device, entry)),
  offline: "",
});

export function viewWatch(state) {
  const columns = {
    device: ({ entry }) => deviceLabel(state, entry),
    name: ({ entry }) => entry.name || objectText(entry),
    value: ({ entry }) => entry.value ?? null,
    state: ({ entry }) => entry.status ?? null,
  };
  const { shown, bar } = filterTable("watch", state.watchFilter, state.watch.map((entry, index) => ({ entry, index })), describe(state));
  const rows = shown.length ? sortRows(shown, state.watchSort, columns).map((row) => rowHtml(state, row)).join("") : `<div class="empty"><p>${t("table.none")}</p></div>`;
  const body = state.watch.length
    ? problemHead("watch", state.watchSort, "<span></span>") + rows
    : `<div class="empty"><h2>${t("watch.emptyTitle")}</h2><p>${t("watch.emptyText")}</p></div>`;
  return {
    title: t("tab.watch"),
    lead: t("watch.lead"),
    next: null,
    html: `<div class="split single"><section class="listwrap">${state.watch.length ? bar : ""}<div class="rows" data-keep="watch">${body}</div></section></div>`,
  };
}
