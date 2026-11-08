// The open device's points as a page to print or save as PDF (docs/concept-toolbox.md part 1): the
// groups, filter and view of the list, without bacnetio's own controls around them.

import { DEVICE_TYPE, chips, deviceMaker, deviceName, pointKey, valueHtml } from "./format.js";
import { typeShort } from "./grouping.js";
import { h } from "./html.js";
import { currentLocale, objectText, objectTypeName, t } from "./i18n.js";
import { heldHere, shownGroups, visiblePoints } from "./points.js";
import { siteName, openSiteOf } from "./sites.js";

/** What narrows the list, in words: the view, the search and the types. */
function narrowedBy(state) {
  return [
    state.pointView !== "all" ? t(`view.${state.pointView}`) : "",
    state.filter.trim() ? `"${state.filter.trim()}"` : "",
    state.pointTypes.map(typeShort).join(", "),
  ].filter(Boolean).join(" · ");
}

function factsHtml(state, points, shownCount, version) {
  const device = state.device;
  const site = openSiteOf(state);
  const readAt = state.readAt[device.instance] ?? state.scan.remembered_ms;
  const narrowed = narrowedBy(state);
  const count = narrowed
    ? t("print.some", { what: h(narrowed), shown: shownCount, n: points.length })
    : points.length === 1 ? t("print.one") : t("print.all", { n: points.length });
  const facts = [
    site ? [t("print.site"), h(siteName(site) ?? site.name)] : null,
    [t("print.device"), `${h(deviceName(device))} <span class="mono">#${device.instance}</span>`],
    [t("print.address"), `<span class="mono">${h(device.address)}</span>`],
    deviceMaker(device) ? [t("print.maker"), h(deviceMaker(device))] : null,
    readAt ? [t("print.read"), h(new Date(readAt).toLocaleString(currentLocale()))] : null,
    [t("print.shown"), count],
    [t("print.made"), `${h(new Date().toLocaleString(currentLocale()))} · ${version}`],
  ].filter(Boolean);
  return `<dl class="print-facts">${facts.map(([term, value]) => `<dt>${term}</dt><dd>${value}</dd>`).join("")}</dl>`;
}

function rowHtml(state, { point, name }, withPriority) {
  const priority = withPriority ? `<td class="mono">${state.winners[pointKey(point)] ?? ""}</td>` : "";
  return `<tr><td class="mono">${typeShort(point.type)} ${point.instance}</td><td>${h(name || objectText(point))}</td>
    <td>${h(point.description ?? "")}</td><td class="mono">${valueHtml(point, point.state_texts)}</td>
    <td>${chips(point, heldHere(state, point))}</td>${priority}</tr>`;
}


function groupTitle(group) {
  if (group.type !== undefined) return objectTypeName(group.type);
  return group.kind ? t(group.title) : group.title;
}

/** The table; the column of the priority now only once it was read, an empty one would look lost. */
function tableHtml(state, groups) {
  const withPriority = state.winnersRead;
  const head = `<thead><tr><th>${t("col.type")}</th><th>${t("col.name")}</th><th>${t("description")}</th><th>${t("col.value")}</th><th>${t("col.state")}</th>${withPriority ? `<th>${t("print.priority")}</th>` : ""}</tr></thead>`;
  const bodies = groups.map((group) => `<tbody><tr class="print-group"><th colspan="${withPriority ? 6 : 5}">${h(groupTitle(group))} <span class="sub">${group.items.length}</span></th></tr>
    ${group.items.map((item) => rowHtml(state, item, withPriority)).join("")}</tbody>`).join("");
  return `<table class="print-table">${head}${bodies}</table>`;
}

export function viewPrint(state) {
  const points = state.points.filter((point) => point.type !== DEVICE_TYPE);
  const groups = shownGroups(state, points, visiblePoints(state));
  const shownCount = groups.reduce((sum, group) => sum + group.items.length, 0);
  const version = state.version ? `bacnetio ${h(state.version)}` : "bacnetio";
  const list = shownCount ? tableHtml(state, groups) : `<p>${t("filter.none")}</p>`;
  return {
    title: deviceName(state.device),
    printing: true,
    html: `<article class="print-page">
      <div class="print-acts"><button type="button" data-act="print-now">${t("print.now")}</button>
        <button type="button" class="quiet" data-act="print-close">${t("print.back")}</button>
        ${state.winnersRead ? "" : `<button type="button" class="quiet" data-act="read-winners" title="${t("winners.hint")}"${state.reading || state.readingWinners ? " disabled" : ""}>${t(state.readingWinners ? "winners.reading" : "winners.read")}</button>`}
        <span class="sub">${t("print.hint")}</span></div>
      <h1>${h(deviceName(state.device))}</h1>
      ${factsHtml(state, points, shownCount, version)}
      <div class="print-scroll">${list}</div>
    </article>`,
  };
}
