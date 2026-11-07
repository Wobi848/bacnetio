// Two devices side by side, their points matched by name (docs/concept-toolbox.md part 2): two air
// handling units of one kind should be alike, and this shows where they are not.

import { DEVICE_TYPE, chips, deviceMaker, deviceName, pointKey, valueHtml } from "./format.js";
import { typeShort } from "./grouping.js";
import { h } from "./html.js";
import { objectText, t } from "./i18n.js";
import { listBorders } from "./panels.js";

/** Where a name may be cut: the shared start ends at one of these. */
const SEPARATOR = /[/\\._\s-]/;

/** The start all names share, up to its last separator, such as "AHU-01 " or "/21/000/". */
export function sharedStart(names) {
  if (names.length < 2) return "";
  let length = 0;
  const [first] = names;
  while (length < first.length && names.every((name) => name[length] === first[length])) length++;
  while (length > 0 && !SEPARATOR.test(first[length - 1])) length--;
  return first.slice(0, length);
}

/** A device's points by the name they are matched with: without the start all its names share. */
function byMatchName(points) {
  const named = points.filter((point) => point.type !== DEVICE_TYPE);
  const start = sharedStart(named.map((point) => point.name ?? ""));
  return named.map((point) => ({ point, match: (point.name ?? "").slice(start.length).trim() || objectText(point) }));
}

const unitOf = (point) => point.units ?? "";
const valueOf = (point) => `${point.value ?? ""}`;
const stateOf = (point) => [...(point.status ?? "").split(",")].map((flag) => flag.trim()).filter(Boolean).sort().join(",");

/** What differs between two points matched, in the words of the tooltip. */
function differences(left, right) {
  return [
    left.type !== right.type ? "type" : "",
    valueOf(left) !== valueOf(right) ? "value" : "",
    unitOf(left) !== unitOf(right) ? "unit" : "",
    stateOf(left) !== stateOf(right) ? "state" : "",
  ].filter(Boolean);
}

/**
 * Rows `{ name, left, right, differs }`: the left device's points in their order, each with the
 * right device's point of the same name, then the points only the right device has.
 */
export function matchPoints(leftPoints, rightPoints) {
  const waiting = new Map();
  for (const item of byMatchName(rightPoints)) {
    if (!waiting.has(item.match)) waiting.set(item.match, []);
    waiting.get(item.match).push(item.point);
  }
  const rows = byMatchName(leftPoints).map(({ point, match }) => {
    const right = waiting.get(match)?.shift() ?? null;
    return { name: match, left: point, right, differs: right ? differences(point, right) : [] };
  });
  for (const [match, points] of waiting) {
    for (const point of points) rows.push({ name: match, left: null, right: point, differs: [] });
  }
  return rows;
}

/** The views of the comparison, each a question asked of a row. */
export const COMPARE_VIEWS = {
  all: () => true,
  differ: (row) => row.left && row.right && row.differs.length > 0,
  onlyLeft: (row) => !row.right,
  onlyRight: (row) => !row.left,
};

function sideHtml(device, point, differs) {
  if (!point) return `<td class="compare-none" colspan="3">${t("compare.missing")}</td>`;
  const mark = (what) => (differs.includes(what) ? " differs" : "");
  const title = differs.length ? ` title="${h(differs.map((what) => t(`compare.diff.${what}`)).join(", "))}"` : "";
  const open = `data-act="compare-open" data-device="${device.instance}" data-key="${h(pointKey(point))}"`;
  return `<td class="mono${mark("type")}"><button type="button" class="plain" ${open}>${typeShort(point.type)} ${point.instance}</button></td>
    <td class="mono${mark("value")}${mark("unit")}"${title}>${valueHtml(point, point.state_texts)}</td>
    <td class="${mark("state")}">${chips(point, false)}</td>`;
}

function rowHtml(left, right, row) {
  return `<tr${row.differs.length ? ' class="differs"' : ""}><th scope="row">${h(row.name)}</th>${sideHtml(left, row.left, row.differs)}${sideHtml(right, row.right, row.differs)}</tr>`;
}

function chooserHtml(state) {
  const device = state.device;
  const maker = deviceMaker(device);
  const others = (state.scan.devices ?? [])
    .filter((other) => other.instance !== device.instance)
    .sort((one, two) => Number(deviceMaker(two) === maker) - Number(deviceMaker(one) === maker) || deviceName(one).localeCompare(deviceName(two)));
  const buttons = others.map((other) => {
    const read = state.sitePoints[other.instance] ? "" : ` <span class="sub">${t("compare.notRead")}</span>`;
    return `<li><button type="button" class="quiet" data-act="compare-choose" data-device="${other.instance}">${h(deviceName(other))} <span class="mono sub">#${other.instance}</span></button> <span class="sub">${h(deviceMaker(other))}</span>${read}</li>`;
  }).join("");
  const list = buttons ? `<ul class="compare-choices">${buttons}</ul>` : `<p>${t("compare.noOther")}</p>`;
  return `<section class="compare-panel" aria-label="${t("compare.choose")}"><h3>${t("compare.choose")}</h3>
    <p class="hint">${t("compare.lead", { name: h(deviceName(device)) })}</p>${list}
    <span class="acts"><button type="button" class="quiet" data-act="compare-close">${t("cancel")}</button></span></section>`;
}

function toolbarHtml(state, rows) {
  const views = Object.keys(COMPARE_VIEWS).map((view) => {
    const count = rows.filter(COMPARE_VIEWS[view]).length;
    return `<button type="button" data-act="compare-view" data-view="${view}" aria-pressed="${state.compare.view === view}">${t(`compare.view.${view}`)} <span class="n${view !== "all" && count ? " warn" : ""}">${count}</span></button>`;
  }).join("");
  const busy = state.compare.reading ? " disabled" : "";
  return `<div class="toolbar"><span class="seg">${views}</span>
    <div class="end"><button type="button" class="quiet" data-act="compare-read"${busy}>${t(state.compare.reading ? "compare.reading" : "compare.read")}</button>
    <button type="button" class="quiet" data-act="compare-close">${t("compare.close")}</button></div></div>`;
}

/** The comparison in place of the point list; the chooser while no second device is chosen. */
export function compareHtml(state) {
  if (state.compare.other === null) return `<section class="listwrap">${listBorders()}${chooserHtml(state)}</section>`;
  const right = state.scan.devices?.find((device) => device.instance === state.compare.other);
  const rightPoints = state.sitePoints[state.compare.other]?.points;
  if (!right || !rightPoints) {
    return `<section class="listwrap">${listBorders()}<div class="empty"><p><span class="dot busy"></span> ${t("compare.readingOther", { name: h(right ? deviceName(right) : `#${state.compare.other}`) })}</p></div></section>`;
  }
  const rows = matchPoints(state.points ?? [], rightPoints);
  const shown = rows.filter(COMPARE_VIEWS[state.compare.view]);
  const head = (device) => `<th colspan="3" scope="col">${h(deviceName(device))} <span class="mono sub">#${device.instance}</span></th>`;
  const body = shown.length
    ? shown.map((row) => rowHtml(state.device, right, row)).join("")
    : `<tr><td colspan="7">${t("compare.none")}</td></tr>`;
  return `<section class="listwrap">${listBorders()}${toolbarHtml(state, rows)}
    <div class="rows compare-wrap" data-keep="compare"><table class="compare-table">
      <thead><tr><th scope="col">${t("col.name")}</th>${head(state.device)}${head(right)}</tr></thead>
      <tbody>${body}</tbody></table></div></section>`;
}
