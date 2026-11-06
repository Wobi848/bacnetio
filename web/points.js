// The data points of the open device: grouped by plant part, searchable, filtered by problem or
// change since the reference, with the chosen point's details beside the list.

import { deviceDetailHtml } from "./devices.js";
import { DEVICE_TYPE, RECORDABLE, chips, deviceMaker, deviceName, errorStrip, has, isFault, isHeldEntry, isOverridden, isRecorded, isWatched, noteStrip, pointKey, valueHtml } from "./format.js";
import { groupPoints, typeShort } from "./grouping.js";
import { h, marked } from "./html.js";
import { currentLocale, objectText, objectTypeName, t } from "./i18n.js";
import { listBorders } from "./panels.js";
import { detailHtml } from "./point-detail.js";
import { pluginMarksHtml } from "./plugins.js";
import { matchesQuery, parseQuery } from "./point-query.js";
import { AREA_ICONS, headToolHtml, readingText } from "./shell.js";
import { sortHead, sortRows } from "./sorting.js";
import { isViewNode, nodePoints, plantNodes } from "./structure.js";

/** Rows open when a device is opened: the first thing to see is values, few enough to stay quick. */
const OPEN_ROWS = 200;
/** After this the values on screen count as old. */
const OLD_AFTER_MS = 5 * 60 * 1000;
/** Rows of a flat list until all are asked for. */
const MOST_ROWS = 1000;
/** Notes about how the device was read that matter to the user; the rest is technical. */
const SHOWN_NOTES = ["no_read_property_multiple", "stopped_answering", "stopped_by_user"];

const POINT_COLUMNS = {
  name: ({ name, point }) => name || objectText(point),
  description: ({ point }) => point.description,
  id: ({ point }) => point.instance,
  type: ({ point }) => typeShort(point.type),
  value: ({ point }) => point.value,
  state: ({ point }) => point.status,
};

export function heldHere(state, point) {
  return state.held.some((entry) => isHeldEntry(entry, state.device?.instance, point));
}

function changesOf(state, point) {
  return (state.reference?.changes ?? []).filter((change) => change.object_type === point.type && change.instance === point.instance && !change.drift);
}

/** The points of the chosen plant node, or all of the device's. */
function nodeOrAll(state) {
  const points = (state.points ?? []).filter((point) => point.type !== DEVICE_TYPE);
  if (!isViewNode(state.node)) return points;
  const nodes = plantNodes(state.structure[state.device?.instance] ?? []);
  return nodePoints(nodes, state.node, points);
}

/** The points of the chosen node of the tree, whatever kind of node it is: what the counts count. */
function pointsOfNode(state) {
  const points = nodeOrAll(state);
  const node = state.node;
  if (!node || isViewNode(node)) return points;
  if (node.startsWith("type:")) return points.filter((point) => `type:${point.type}` === node);
  const group = groupPoints(points).find((found) => found.key === node);
  return group ? group.items.map((item) => item.point) : points;
}

/** The views of the list, each a question asked of a point. */
const VIEWS = {
  all: () => true,
  faults: (state, point) => isFault(point),
  overridden: (state, point) => isOverridden(point, heldHere(state, point)),
  alarm: (state, point) => has(point, "alarm"),
  outOfService: (state, point) => has(point, "out of service"),
  held: (state, point) => heldHere(state, point),
  changed: (state, point) => changesOf(state, point).length > 0,
};
/** Views shown only while some point is in them, so the toolbar stays short. */
const RARE_VIEWS = { alarm: "err", outOfService: "warn", held: "warn" };

/** The points the search, the chosen view and the chosen types let through. */
export function visiblePoints(state) {
  const query = parseQuery(state.filter);
  return nodeOrAll(state)
    .filter((point) => VIEWS[state.pointView](state, point))
    .filter((point) => !state.pointTypes.length || state.pointTypes.includes(point.type))
    .filter((point) => !state.node?.startsWith("type:") || `type:${point.type}` === state.node)
    .filter((point) => matchesQuery(point, query));
}

function viewButton(state, view, count, kind = "") {
  return `<button type="button" data-act="point-view" data-view="${view}" aria-pressed="${state.pointView === view}">${t(`view.${view}`)} <span class="n ${count ? kind : ""}">${count}</span></button>`;
}

/** Only the types the device has, each with its count, to tick several at once. */
function typesMenu(state, points) {
  const counts = new Map();
  for (const point of points) counts.set(point.type, (counts.get(point.type) ?? 0) + 1);
  const open = state.menu === "types";
  const chosen = state.pointTypes.length ? ` · ${state.pointTypes.map(typeShort).join(", ")}` : "";
  const items = [...counts].sort(([first], [second]) => first - second).map(([type, count]) =>
    `<label class="check"><input type="checkbox" data-act="point-type" data-type="${type}"${state.pointTypes.includes(type) ? " checked" : ""}><span class="mono sub">${typeShort(type)}</span><span>${h(objectTypeName(type))}</span><span class="n">${count}</span></label>`).join("");
  const menu = open ? `<div class="menu types" role="group" aria-label="${t("filter.types")}">${items}</div>` : "";
  return `<span class="menu-host"><button type="button" class="quiet${chosen ? " chosen" : ""}" data-act="types-menu" aria-expanded="${open}">${t("filter.types")}${h(chosen)}</button>${menu}</span>`;
}

/** What narrows the list now, each removable on its own. */
function activeFilters(state) {
  const removable = (kind, label, value = "") =>
    `<button type="button" class="chip removable" data-act="filter-remove" data-kind="${kind}" data-value="${value}" title="${t("filter.remove")}">${h(label)} <span aria-hidden="true">×</span></button>`;
  const chips = [
    state.filter.trim() ? removable("search", `"${state.filter.trim()}"`) : "",
    state.pointView !== "all" ? removable("view", t(`view.${state.pointView}`)) : "",
    ...state.pointTypes.map((type) => removable("type", typeShort(type), type)),
  ].join("");
  if (!chips) return "";
  return `<div class="filters">${chips}<button type="button" class="quiet" data-act="filters-clear">${t("filter.clearAll")}</button></div>`;
}

/** What is used now and then, so that the toolbar stays with what is used all the time. */
function moreMenu(state) {
  const device = state.device.instance;
  const menu = state.menu !== "more" ? "" : `<div class="menu end exports" role="menu">
    <button type="button" class="quiet drawer-only" data-act="device-details">${t("dev.details")}</button>
    <hr class="drawer-only">
    <a class="quiet" href="/api/devices/${device}/points.csv" download>CSV</a>
    <a class="quiet" href="/api/devices/${device}/points-priorities.csv" download title="${t("csv.prioritiesHint")}">${t("csv.priorities")}</a>
    <a class="quiet" href="/api/devices/${device}/points.ede.csv" download title="${t("ede.hint")}">EDE</a>
    <a class="quiet" href="/api/devices/${device}/points.ede-states.csv" download title="${t("ede.statesHint")}">${t("ede.states")}</a>
    <hr>
    <button type="button" class="quiet" data-act="groups-all" data-open="true">${t("groups.open")}</button>
    <button type="button" class="quiet" data-act="groups-all" data-open="false">${t("groups.close")}</button>
    <hr>
    <button type="button" class="quiet" data-act="ref-save" title="${t("ref.hint")}">${t(state.reference?.time_ms ? "ref.saveNew" : "ref.save")}</button>
    <button type="button" class="quiet" data-act="read-winners" title="${t("winners.hint")}"${state.reading || state.readingWinners ? " disabled" : ""}>${t(state.readingWinners ? "winners.reading" : "winners.read")}</button></div>`;
  return `<span class="menu-host"><button type="button" class="quiet" data-act="more-menu" aria-expanded="${state.menu === "more"}">${t("more")}</button>${menu}</span>`;
}

/** When the values on screen were read: live values need no such word. */
function readAtChip(state) {
  const readAt = state.readAt[state.device.instance] ?? state.scan.remembered_ms;
  if (!readAt || state.liveList) return "";
  const time = new Date(readAt).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });
  const age = Date.now() - readAt;
  if (age < OLD_AFTER_MS) return `<span class="chip">${t("read.at", { time })}</span>`;
  return `<span class="chip warn">${t("read.ago", { time, n: Math.round(age / 60000) })}</span>`;
}

function toolbar(state, points, also) {
  const count = (view) => points.filter((point) => VIEWS[view](state, point)).length;
  const rare = Object.entries(RARE_VIEWS).map(([view, kind]) => {
    const n = count(view);
    return n || state.pointView === view ? viewButton(state, view, n, kind) : "";
  }).join("");
  const reference = state.reference?.time_ms
    ? viewButton(state, "changed", count("changed"), "warn")
    : "";
  const referenceTitle = state.reference?.time_ms
    ? ` title="${h(t("ref.since", { when: new Date(state.reference.time_ms).toLocaleString(currentLocale()) }))}"`
    : "";
  return `<div class="toolbar">
    <button type="button" class="quiet phone-back" data-act="tree-site">← ${t("tab.devices")}</button>
    <input id="filter" type="search" title="${t("filter.hint")} (f)" placeholder="${t("filter")}" value="${h(state.filter)}" autocomplete="off">
    <span class="seg"${referenceTitle}>${viewButton(state, "all", points.length)}${viewButton(state, "faults", count("faults"), "err")}${viewButton(state, "overridden", count("overridden"), "warn")}${rare}${reference}</span>
    ${readAtChip(state)}
    <div class="end">
      ${headToolHtml(also)}
      ${typesMenu(state, points)}
      <button type="button" class="quiet" data-act="reload"${state.reading ? " disabled" : ""}>${t("readAgain")}</button>
      <button type="button" class="${state.liveList ? "tonal" : "quiet"}" data-act="live-list" aria-pressed="${state.liveList}" title="${t("live.hint")}">${t(state.liveList ? "live.on" : "live.off")}</button>
      ${moreMenu(state)}
    </div>
    ${activeFilters(state)}</div>`;
}

function pointRow(state, item, inGroup) {
  const { point, name } = item;
  const active = state.selectedPoint === pointKey(point) ? " active" : "";
  const was = changesOf(state, point).map((change) => {
    if (change.kind === "added") return `<span class="chip warn">${t("ref.added")}</span>`;
    return `<span class="chip warn">${t("ref.was", { was: h(change.was) })}</span>`;
  }).join("");
  const wins = state.winners[pointKey(point)];
  const winner = wins ? `<span class="chip accent">${t("chip.priority", { n: wins })}</span>` : "";
  const { words } = parseQuery(state.filter);
  return `<div class="row points${active}" data-point="${pointKey(point)}" role="option" aria-selected="${Boolean(active)}" title="${h(point.name ?? "")}">
    <span class="name${inGroup ? " indent" : ""}">${marked(name || objectText(point), words)}${pluginMarksHtml(state, state.device.instance, point.type, point.instance)}</span><span class="desc cut" title="${h(point.description ?? "")}">${marked(point.description ?? "", words)}</span>
    <span class="id mono sub">${point.instance}</span><span class="type mono sub">${typeShort(point.type)}</span>
    <span class="value mono">${valueHtml(point, point.state_texts)}</span><span class="chips">${winner}${chips(point, heldHere(state, point))}${was}${quickHtml(state, point)}</span></div>`;
}

/** Watch and record from the row, without opening the details: filled when on. */
function quickHtml(state, point) {
  if (!RECORDABLE.includes(point.type)) return "";
  const device = state.device.instance;
  const button = (act, area, on, label) =>
    `<button type="button" class="plain quick${on ? " on" : ""}" data-act="${act}" data-key="${pointKey(point)}" aria-pressed="${on}" title="${label}" aria-label="${label}">${AREA_ICONS[area]}</button>`;
  const watched = isWatched(state, device, point);
  const recorded = isRecorded(state, device, point);
  return `<span class="quicks">${button("quick-watch", "watch", watched, t(watched ? "watch.remove" : "watch.add"))}${button("quick-record", "recording", recorded, t(recorded ? "rec.removeHere" : "rec.add"))}</span>`;
}

function groupRow(state, group, open) {
  const faults = group.items.filter(({ point }) => isFault(point)).length;
  const overridden = group.items.filter(({ point }) => isOverridden(point, heldHere(state, point))).length;
  const outOfService = group.items.filter(({ point }) => has(point, "out of service")).length;
  const marks = [
    faults ? `<span class="chip err">${faults === 1 ? t("ov.oneFault") : t("ov.faults", { n: faults })}</span>` : "",
    overridden ? `<span class="chip warn">${t("ov.overridden", { n: overridden })}</span>` : "",
    outOfService ? `<span class="chip warn">${t("ov.outOfService", { n: outOfService })}</span>` : "",
  ].join("");
  const title = group.type !== undefined ? objectTypeName(group.type) : group.kind ? t(group.title) : group.title;
  return `<div class="row group${open ? " open" : ""}" data-group="${h(group.key)}" role="button" aria-expanded="${open}">
    <span class="arrow">▶</span><span class="name">${h(title)}</span><span class="count">${marks}${group.items.length}</span></div>`;
}

function listHtml(state, shown, points) {
  const empty = `<div class="empty"><p>${t(state.pointView === "changed" ? "ref.none" : "filter.none")}</p><p class="hint">${t("filter.hint")}</p></div>`;
  if (!shown.length) return empty;
  const narrowed = state.filter.trim() !== "" || state.pointView !== "all" || state.pointTypes.length > 0;
  // grouped by all the device's points, so that a filter does not rename the plant parts
  const passing = new Set(shown);
  const groups = groupPoints(points)
    .map((group) => ({ ...group, items: group.items.filter(({ point }) => passing.has(point)) }))
    .filter((group) => group.items.length);
  // a plant part is one group; a type or a plant view node is already filtered out of `shown`
  const byGroup = state.node && !state.node.startsWith("type:") && !isViewNode(state.node);
  const chosen = byGroup ? groups.filter((group) => group.key === state.node) : groups;
  if (!chosen.length) return empty;
  const openAll = narrowed || chosen.length === 1;
  let openRows = OPEN_ROWS;
  const sort = state.pointSort;
  const head = `<div class="row points head" role="row">${sortHead(t("col.name"), "points", "name", sort)}${sortHead(t("description"), "points", "description", sort, "desc")}${sortHead(t("col.id"), "points", "id", sort, "id")}${sortHead(t("col.type"), "points", "type", sort)}${sortHead(t("col.value"), "points", "value", sort, "num")}${sortHead(t("col.state"), "points", "state", sort)}</div>`;
  // in a plant view the device's own order counts, so the rows follow the view, not the groups
  const shortNames = new Map(groups.flatMap((group) => group.items.map((item) => [item.point, item.name])));
  const items = isViewNode(state.node)
    ? shown.map((point) => ({ point, name: shortNames.get(point) ?? point.name ?? "" }))
    : chosen.flatMap((group) => group.items);
  // sorted or in one node of the tree, the list is one list: groups would only keep things apart
  if (sort || state.node) {
    const rows = sortRows(items, sort, POINT_COLUMNS);
    // redrawn with every live value: thousands of rows would make each click wait
    const shownRows = state.allRows ? rows : rows.slice(0, MOST_ROWS);
    const more = shownRows.length < rows.length ? `<div class="empty"><button type="button" class="quiet" data-act="all-rows">${t("list.all", { n: rows.length })}</button></div>` : "";
    return head + shownRows.map((item) => pointRow(state, item, false)).join("") + more;
  }
  return head + chosen.map((group) => {
    const open = state.groupsOpen[group.key] ?? state.groupsDefault ?? (openAll || openRows > 0);
    if (open) openRows -= group.items.length;
    return groupRow(state, group, open) + (open ? group.items.map((item) => pointRow(state, item, true)).join("") : "");
  }).join("");
}

function readingHtml(state) {
  if (state.error) return `<div class="strips">${errorStrip(state.error)}</div>`;
  return `<div class="empty"><p><span class="dot busy"></span> ${h(readingText(state.readProgress))}</p></div>`;
}

/** What the page is and what to do here, with the count; the device facts are in its details. */
function pointsLead(state, points) {
  const held = points.filter((point) => heldHere(state, point)).length;
  if (points.length === 1) return t("pts.leadOne");
  return held ? t("pts.leadHeld", { n: points.length, held }) : t("pts.lead", { n: points.length });
}

export function viewPoints(state, also = null) {
  const device = state.device;
  // while the list is read there is nothing to choose yet: the device facts say where it is read from
  const lead = `${h(deviceMaker(device))} · <span class="mono">#${device.instance} · ${h(device.address)}</span>`;
  if (!state.points) return { title: deviceName(device), lead, html: `<section class="listwrap">${listBorders()}${readingHtml(state)}</section>${deviceDetailHtml(state)}` };
  const points = state.points.filter((point) => point.type !== DEVICE_TYPE);
  const notes = state.pointNotes.filter((item) => SHOWN_NOTES.includes(item.code)).map((item) => noteStrip(item)).join("");
  const strips = notes || state.error ? `<div class="strips">${errorStrip(state.error)}${notes}</div>` : "";
  const shown = visiblePoints(state);
  return {
    title: deviceName(device),
    lead: h(pointsLead(state, points)),
    html: `<section class="listwrap">${listBorders()}${toolbar(state, pointsOfNode(state), also)}${strips}<div class="rows${points.some((point) => point.description) ? "" : " no-desc"}" data-keep="points" tabindex="0" role="listbox" data-list="points">${listHtml(state, shown, points)}</div></section>
      ${state.selectedPoint ? detailHtml(state) : deviceDetailHtml(state)}`,
  };
}
