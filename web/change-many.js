// One property, one value, many points of the open device (docs/concept-toolbox.md part 5): the
// points chosen in the list, a table of what changes, one more question, then one write after the
// other, each logged by the server as a single write is.

import { deviceName, has, pointKey, valueHtml } from "./format.js";
import { typeShort } from "./grouping.js";
import { h } from "./html.js";
import { objectText, t } from "./i18n.js";

/** Points per change at most: as many as the live values of a list read at once. */
export const MOST_POINTS = 200;

const ANALOG = [0, 1, 2];
const BINARY = [3, 4, 5];
const MULTI_STATE = [13, 14, 19];
/** Objects whose present value is written with a priority. */
const COMMANDED = [1, 2, 4, 5, 14, 19];

/**
 * What can be changed, in the order offered: the object types that have it and how it is written.
 * `property` and `dataType` as `POST …/property` takes them (BACnet property numbers).
 */
export const CHANGES = {
  value: { types: COMMANDED, priority: true },
  release: { types: COMMANDED, priority: true, noValue: true },
  outOfService: { types: [...ANALOG, ...BINARY, ...MULTI_STATE], property: 81, dataType: "boolean", yesNo: true },
  covIncrement: { types: ANALOG, property: 22, dataType: "real" },
  description: { types: null, property: 28, dataType: "text" },
  highLimit: { types: ANALOG, property: 45, dataType: "real" },
  lowLimit: { types: ANALOG, property: 59, dataType: "real" },
};

const isNumber = (text) => text.trim() !== "" && Number.isFinite(Number(text.trim().replace(",", ".")));
const BINARY_WORDS = ["active", "inactive", "on", "off", "1", "0", "true", "false"];

/** Why a point cannot take the value, as a text key; null when it can. */
export function refusal(kind, point, text) {
  const change = CHANGES[kind];
  if (change.types && !change.types.includes(point.type)) return "many.notThis";
  if (change.noValue || change.yesNo || change.dataType === "text") return null;
  if (change.dataType === "real" || ANALOG.includes(point.type)) return isNumber(text) ? null : "many.needsNumber";
  if (BINARY.includes(point.type)) return BINARY_WORDS.includes(text.trim().toLowerCase()) ? null : "many.needsBinary";
  if (MULTI_STATE.includes(point.type)) {
    const state = Number(text.trim());
    const states = point.state_texts?.length || Infinity;
    return Number.isInteger(state) && state >= 1 && state <= states ? null : "many.needsState";
  }
  return null;
}

/** The value now, where the list knows it; null where it would have to be read. */
function now(kind, point) {
  if (kind === "value") return point.value;
  if (kind === "outOfService") return has(point, "out of service") ? "true" : "false";
  if (kind === "description") return point.description ?? "";
  return null;
}

/** Each chosen point with what happens to it: `{ point, refused, unchanged }`. */
export function plan(kind, points, text) {
  return points.map((point) => {
    const refused = refusal(kind, point, text);
    const before = now(kind, point);
    const unchanged = !refused && !CHANGES[kind].noValue && kind !== "value" && before !== null && before === text;
    return { point, refused, unchanged };
  });
}

export const toWrite = (rows) => rows.filter((row) => !row.refused && !row.unchanged);

const yesNo = (text) => (text === "true" ? t("props.yes") : t("props.no"));

function shownNow(kind, point) {
  if (kind === "value" || kind === "release") return valueHtml(point, point.state_texts);
  const before = now(kind, point);
  if (before === null) return `<span class="sub">${t("many.notRead")}</span>`;
  return h(kind === "outOfService" ? yesNo(before) : before);
}

function shownNew(kind, text, priority) {
  if (kind === "release") return h(t("many.released", { priority }));
  const value = kind === "outOfService" ? yesNo(text) : text;
  return CHANGES[kind].priority ? h(t("many.atPriority", { value, priority })) : h(value);
}

function resultHtml(result) {
  if (!result) return "";
  if (result.state === "done") return `<span class="chip ok">${t("many.done")}</span>`;
  if (result.state === "writing") return `<span class="dot busy"></span>`;
  return `<span class="chip err" title="${h(result.reason)}">${t("many.failed")}</span> <span class="sub">${h(result.reason)}</span>`;
}

function rowHtml(many, row) {
  const { point } = row;
  const note = row.refused ? `<span class="sub">${t(row.refused)}</span>` : row.unchanged ? `<span class="sub">${t("many.unchanged")}</span>` : "";
  return `<tr${row.refused || row.unchanged ? ' class="left-out"' : ""}><td class="mono">${typeShort(point.type)} ${point.instance}</td><td>${h(point.name ?? objectText(point))}</td>
    <td class="mono">${shownNow(many.kind, point)}</td><td class="mono">${row.refused || row.unchanged ? "" : shownNew(many.kind, many.value, many.priority)}</td>
    <td>${note}${resultHtml(many.results[pointKey(point)])}</td></tr>`;
}

function fieldsHtml(many) {
  const kinds = Object.keys(CHANGES).map((kind) => `<option value="${kind}"${kind === many.kind ? " selected" : ""}>${t(`many.kind.${kind}`)}</option>`).join("");
  const change = CHANGES[many.kind];
  let value = "";
  if (change.yesNo) {
    value = `<label class="many-field"><span>${t("many.value")}</span><select id="many-value"><option value="true"${many.value === "true" ? " selected" : ""}>${t("props.yes")}</option><option value="false"${many.value === "false" ? " selected" : ""}>${t("props.no")}</option></select></label>`;
  } else if (!change.noValue) {
    value = `<label class="many-field"><span>${t("many.value")}</span><input id="many-value" value="${h(many.value)}" autocomplete="off"></label>`;
  }
  const priorities = Array.from({ length: 16 }, (_, index) => index + 1)
    .map((priority) => `<option value="${priority}"${priority === many.priority ? " selected" : ""}>${priority}</option>`).join("");
  const priority = change.priority ? `<label class="many-field"><span>${t("many.priority")}</span><select id="many-priority">${priorities}</select></label>` : "";
  return `<label class="many-field"><span>${t("many.property")}</span><select id="many-kind">${kinds}</select></label>${value}${priority}`;
}

function summaryHtml(many) {
  const results = Object.values(many.results);
  const done = results.filter((result) => result.state === "done").length;
  const failed = results.filter((result) => result.state === "failed").length;
  return `<p>${t("many.summary", { done, failed })}</p>`;
}

function actsHtml(state, many, rows) {
  const count = toWrite(rows).length;
  if (many.phase === "writing") return `<button type="button" class="quiet" data-act="many-stop"${many.stopping ? " disabled" : ""}>${t(many.stopping ? "many.stopping" : "many.stop")}</button>`;
  if (many.phase === "done") return `${summaryHtml(many)}<span class="acts"><button type="button" data-act="many-close">${t("many.close")}</button></span>`;
  if (!state.writeUnlocked) {
    return `<div class="ask"><b>${t("unlock.title")}</b><span>${t("unlock.text")}</span>
      <div class="actions"><button type="button" data-act="unlock">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="many-close">${t("cancel")}</button></div></div>`;
  }
  if (many.phase === "confirm") {
    const what = shownNew(many.kind, many.value, many.priority);
    return `<div class="ask"><b>${t("many.confirmTitle")}</b><span>${t("many.confirm", { property: t(`many.kind.${many.kind}`), value: what, n: count, device: h(deviceName(state.device)) })}</span>
      <div class="actions"><button type="button" data-act="many-write">${t("many.write", { n: count })}</button><button type="button" class="quiet" data-act="many-back">${t("many.back")}</button></div></div>`;
  }
  return `<span class="acts"><button type="button" data-act="many-ask"${count ? "" : " disabled"}>${t("many.change", { n: count })}</button><button type="button" class="quiet" data-act="many-close">${t("cancel")}</button></span>`;
}

/** The change in place of the point list. */
export function changeManyHtml(state, points) {
  const many = state.changeMany;
  const chosen = points.filter((point) => state.selected.includes(pointKey(point)));
  const rows = plan(many.kind, chosen, many.value);
  const editing = many.phase === "edit";
  return `<section class="many-panel" aria-label="${t("many.title")}">
    <h3>${t("many.title")}</h3>
    <p class="hint">${t("many.lead", { n: chosen.length })}</p>
    ${editing ? `<div class="many-fields">${fieldsHtml(many)}</div>` : ""}
    <div class="many-scroll" data-keep="many"><table class="many-table">
      <thead><tr><th>${t("col.type")}</th><th>${t("col.name")}</th><th>${t("many.now")}</th><th>${t("many.new")}</th><th></th></tr></thead>
      <tbody>${rows.map((row) => rowHtml(many, row)).join("")}</tbody></table></div>
    ${actsHtml(state, many, rows)}
  </section>`;
}
