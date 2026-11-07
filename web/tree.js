// The tree beside the table: network, devices (behind their router), and for a device its plant
// parts and object types. Faults and overrides show on a closed device too.

import { DEVICE_TYPE, deviceName, isFault, isHeldEntry, isOverridden, ownObjects } from "./format.js";
import { groupPoints, sitePattern, typeShort } from "./grouping.js";
import { h } from "./html.js";
import { pluginMarksHtml } from "./plugins.js";
import { objectTypeName, t } from "./i18n.js";
import { searchScope } from "./shell.js";
import { openSiteOf } from "./sites.js";
import { OUTSIDE, nodePoints, outsidePoints, plantNodes } from "./structure.js";

/** What is known about a device's points: the open device's, or what was read before. */
function knownPoints(state, device) {
  if (state.device?.instance === device.instance && state.points) return state.points;
  return state.sitePoints[device.instance]?.points ?? null;
}

/** The views the device carries itself, read when it was opened; none for every other device. */
function knownViews(state, device) {
  if (state.device?.instance !== device.instance) return null;
  const views = state.structure[device.instance];
  return views?.length ? views : null;
}

function marksHtml(state, device, points) {
  if (device.objects?.error) return `<span class="chip err">${t("chip.offline")}</span>`;
  const objects = (points ?? []).filter((point) => point.type !== DEVICE_TYPE);
  const faults = objects.filter(isFault).length;
  const overridden = objects.filter((point) => isOverridden(point, state.held.some((entry) => isHeldEntry(entry, device.instance, point)))).length;
  return `${faults ? `<span class="chip err" title="${t("ov.faults", { n: faults })}">${faults}</span>` : ""}${overridden ? `<span class="chip warn" title="${t("ov.overridden", { n: overridden })}">${overridden}</span>` : ""}`;
}

const nodeHtml = (level, active, attributes, label, end = "") =>
  `<div class="node level-${level}${active ? " active" : ""}" role="treeitem" aria-selected="${active}" ${attributes}><span class="label">${label}</span><span class="end">${end}</span></div>`;

const headingHtml = (label, end = "", title = "") =>
  `<div class="node level-2 heading" role="presentation"${title ? ` title="${h(title)}"` : ""}><span class="label">${label}</span><span class="end">${end}</span></div>`;

/** Why a device shows no plant parts: a sentence that wraps, not a heading that is cut off. */
const plantNoteHtml = (text) => `<p class="node level-2 tree-note" role="presentation">${h(text)}</p>`;

/** One plant part or view: its marks, what it holds, and what sits on another device. */
function partHtml(state, device, key, level, title, items, extra = "") {
  const isOpen = state.device?.instance === device.instance;
  const choose = `data-act="tree-node" data-device="${device.instance}" data-node="${h(key)}"`;
  const end = `${extra}${marksHtml(state, device, items)}<span class="count">${items.length}</span>`;
  return nodeHtml(level, isOpen && state.node === key, choose, h(title), end);
}

/** The device's own plant view: a node per Structured View, then what no view holds. */
function plantHtml(state, device, views, objects) {
  const nodes = plantNodes(views);
  const rows = nodes.map((node) => {
    const elsewhere = node.elsewhere ? `<span class="chip" title="${t("tree.elsewhereHint")}">${t("tree.elsewhere", { n: node.elsewhere })}</span>` : "";
    const failed = node.error ? `<span class="chip err" title="${h(node.error)}">${t("readError")}</span>` : "";
    return partHtml(state, device, node.key, 2 + node.depth, node.title, nodePoints(nodes, node.key, objects), `${failed}${elsewhere}`);
  });
  const outside = outsidePoints(nodes, objects);
  const rest = outside.length ? partHtml(state, device, OUTSIDE, 2, t("tree.outside"), outside) : "";
  return `${headingHtml(t("tree.plant"), "", t("tree.plantHint"))}${rows.join("")}${rest}`;
}

/** The plant parts, from the device's own views or from its object names, and the object types. */
function childrenHtml(state, device, points) {
  const isOpen = state.device?.instance === device.instance;
  const objects = (points ?? []).filter((point) => point.type !== DEVICE_TYPE);
  const views = knownViews(state, device);
  let parts = "";
  if (views) {
    parts = plantHtml(state, device, views, objects);
  } else if (objects.length) {
    parts = groupPoints(objects, sitePattern(openSiteOf(state))).filter((group) => !group.kind).map((group) =>
      partHtml(state, device, group.key, 2, group.title, group.items.map(({ point }) => point))).join("");
  }
  const counts = new Map();
  if (points) for (const point of objects) counts.set(point.type, (counts.get(point.type) ?? 0) + 1);
  else for (const [type, count] of Object.entries(device.objects?.types ?? {})) if (Number(type) !== DEVICE_TYPE) counts.set(Number(type), count);
  const types = [...counts].sort(([first], [second]) => first - second).map(([type, count]) =>
    nodeHtml(3, isOpen && state.node === `type:${type}`, `data-act="tree-node" data-device="${device.instance}" data-node="type:${type}"`, `<span class="type mono">${typeShort(type)}</span> ${h(objectTypeName(type))}`, `<span class="count">${count}</span>`));
  if (state.treeView === "plant") return parts || plantNoteHtml(t(points ? "tree.plantNone" : "tree.plantEmpty"));
  return types.join("");
}

function deviceHtml(state, device) {
  const isOpen = state.device?.instance === device.instance;
  const points = knownPoints(state, device);
  const expanded = isOpen || state.treeOpen[device.instance] === true;
  const hasChildren = Boolean(points?.length || Object.keys(device.objects?.types ?? {}).length);
  const arrow = hasChildren
    ? `<span class="arrow" data-act="tree-toggle" data-device="${device.instance}" role="button" aria-label="${t("tree.toggle")}" aria-expanded="${expanded}">${expanded ? "▾" : "▸"}</span>`
    : `<span class="arrow"></span>`;
  const count = ownObjects(device.objects?.total) ?? null;
  const row = `<div class="node level-1${isOpen && !state.node ? " active" : ""}" role="treeitem" aria-selected="${isOpen && !state.node}" data-act="tree-device" data-device="${device.instance}">
    ${arrow}<span class="label">${h(deviceName(device))}</span><span class="end">${pluginMarksHtml(state, device.instance)}${marksHtml(state, device, points)}${count === null ? "" : `<span class="count">${count}</span>`}</span></div>`;
  return row + (expanded ? childrenHtml(state, device, points) : "");
}

export function treeHtml(state) {
  const devices = state.scan.devices ?? [];
  const direct = devices.filter((device) => device.network === null || device.network === undefined);
  const routed = new Map();
  for (const device of devices.filter((found) => !direct.includes(found))) {
    const key = `${device.address} → ${device.network}`;
    routed.set(key, [...(routed.get(key) ?? []), device]);
  }
  const behind = [...routed].map(([, list]) => `<div class="node level-1 heading" role="presentation"><span class="label">${t("tree.router", { address: h(list[0].address), network: list[0].network })}</span></div>${list.map((device) => deviceHtml(state, device)).join("")}`).join("");
  // looks like a button: the way back to the device list was hard to find as a plain line
  const scope = searchScope(state);
  const root = nodeHtml(0, !state.device, 'data-act="tree-site" role="button"', `${t("tree.allDevices")}${scope ? ` <span class="sub mono">${h(scope)}</span>` : ""}`, `<span class="count">${devices.length || ""}</span>`);
  const view = (name, label) => `<button type="button" data-act="tree-view" data-view="${name}" aria-pressed="${state.treeView === name}">${label}</button>`;
  const views = `<div class="tree-views"><span class="seg">${view("type", t("tree.byType"))}${view("plant", t("tree.plantView"))}</span></div>`;
  return `<nav class="tree" data-keep="tree" role="tree" aria-label="${t("tree.label")}">${views}${root}${direct.map((device) => deviceHtml(state, device)).join("")}${behind}</nav>`;
}
