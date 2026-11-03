// Search and filters of the site-wide tables, site problems and the watch list: the search of the
// point list, the states as views with their counts, and one device or all of them.

import { has, isOverridden } from "./format.js";
import { h } from "./html.js";
import { t } from "./i18n.js";
import { matchesQuery, parseQuery } from "./point-query.js";

/** Each view: its label (the chip's), its colour, and the question it asks of a row. */
const VIEWS = {
  alarm: ["chip.alarm", "err", ({ point }) => Boolean(point) && has(point, "alarm")],
  fault: ["chip.fault", "err", ({ point }) => Boolean(point) && has(point, "fault")],
  readError: ["readError", "err", ({ point }) => Boolean(point?.error)],
  offline: ["chip.offline", "err", ({ offline }) => Boolean(offline)],
  overridden: ["chip.overridden", "warn", ({ point, held }) => Boolean(point) && isOverridden(point, held)],
  outOfService: ["chip.outOfService", "warn", ({ point }) => Boolean(point) && has(point, "out of service")],
};

/** Which state field holds the filter of each table. */
export const TABLE_FILTERS = { problems: "problemFilter", watch: "watchFilter" };

export const noTableFilter = () => ({ text: "", view: "all", device: null });

/** A row by its search, apart from the view. `row`: { deviceInstance, deviceName, point, held, offline }. */
function found(row, filter, query) {
  if (filter.device !== null && row.deviceInstance !== filter.device) return false;
  // a word may name the device; the others go to the point as in the point list
  const words = query.words.filter((word) => !row.deviceName.toLowerCase().includes(word));
  if (!row.point) return !query.instance && !query.types.length && !query.value && words.every((word) => row.offline.toLowerCase().includes(word));
  return matchesQuery(row.point, { ...query, words });
}

/**
 * The rows the filter lets through, and the bar to change it. `describe` tells of a row its device
 * and point; the counts of the views follow the search and the device.
 */
export function filterTable(list, filter, rows, describe) {
  const query = parseQuery(filter.text);
  const searched = rows.filter((row) => found(describe(row), filter, query));
  const shown = filter.view === "all" ? searched : searched.filter((row) => VIEWS[filter.view][2](describe(row)));
  return { shown, bar: barHtml(list, filter, rows.map(describe), searched.map(describe)) };
}

function barHtml(list, filter, all, searched) {
  const view = (name, label, count, kind = "") =>
    `<button type="button" data-act="table-view" data-list="${list}" data-view="${name}" aria-pressed="${filter.view === name}">${t(label)} <span class="n ${count ? kind : ""}">${count}</span></button>`;
  const views = Object.entries(VIEWS).map(([name, [label, kind, test]]) => {
    const count = searched.filter(test).length;
    return count || filter.view === name ? view(name, label, count, kind) : "";
  }).join("");
  const devices = [...new Map(all.map((row) => [row.deviceInstance, row.deviceName])).entries()];
  const options = devices.map(([instance, name]) => `<option value="${instance}"${filter.device === instance ? " selected" : ""}>${h(name)}</option>`).join("");
  const device = devices.length > 1 || filter.device !== null
    ? `<select id="table-device" data-list="${list}" aria-label="${t("table.allDevices")}"><option value="">${t("table.allDevices")}</option>${options}</select>` : "";
  const narrowed = filter.text.trim() || filter.view !== "all" || filter.device !== null;
  const clear = narrowed ? `<button type="button" class="quiet" data-act="table-clear" data-list="${list}">${t("filter.clearAll")}</button>` : "";
  return `<div class="toolbar"><input id="table-filter" data-list="${list}" type="search" title="${t("filter.hint")}" placeholder="${t("filter")}" value="${h(filter.text)}" autocomplete="off">
    <span class="seg">${view("all", "view.all", searched.length)}${views}</span>${device}${clear}</div>`;
}
