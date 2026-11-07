// The log: values you still hold in the plant, with release, and every write of this session.

import { DEVICE_TYPE, deviceName } from "./format.js";
import { h } from "./html.js";
import { currentLocale, objectText, t, valueWord } from "./i18n.js";

/** Joining and leaving an MS/TP trunk are logged under no device (docs/concept-mstp.md). */
const TRUNK_DEVICE = 4194303;
const TRUNK_ENTRY = /^(Joined|Left) the MS\/TP trunk on (\S+) at (\d+) baud as station (\d+)$/;

/** "Joined the trunk on COM3 at 38400 baud as station 9" in the interface's language. */
function trunkText(text) {
  const [, done, port, baud, station] = text.match(TRUNK_ENTRY) ?? [];
  if (!done) return text;
  return t(done === "Joined" ? "log.joined" : "log.left", { port, baud, station });
}

/** The device's name as found, or its number. */
function ownerName(state, entry) {
  if (entry.device === TRUNK_DEVICE) return t("log.trunk");
  const device = state.scan.devices?.find((found) => found.instance === entry.device);
  return device ? deviceName(device) : t("device", { n: entry.device });
}

/** "AHU-01 · Heating valve" for a held or logged entry, with the names known from reading. */
function entryLabel(state, entry) {
  // the clock belongs to the device itself: its name once is enough
  if (entry.type === DEVICE_TYPE) return ownerName(state, entry);
  return `${ownerName(state, entry)} · ${pointName(state, entry)}`;
}

/** The point's name as read, or its object: what the user wrote to. */
function pointName(state, entry) {
  if (entry.type === DEVICE_TYPE) return ownerName(state, entry);
  return state.objectNames[`${entry.device}:${entry.type}:${entry.instance}`] ?? objectText(entry);
}

/** Below the name: the device and the object, unless the entry is the device itself. */
function pointPlace(state, entry) {
  return entry.type === DEVICE_TYPE ? "" : `${ownerName(state, entry)} · ${objectText(entry)}`;
}

/** When it was written: a value held for days may no longer fit the plant. */
function heldSince(entry) {
  const time = new Date(entry.time * 1000);
  const today = time.toDateString() === new Date().toDateString();
  const clock = time.toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });
  return today ? clock : `${time.toLocaleDateString(currentLocale())} ${clock}`;
}

/** The value written and the unit the point list read for it: ["50", "%"], ["On", ""]. */
function writtenValue(state, entry) {
  const unit = state.objectUnits[`${entry.device}:${entry.type}:${entry.instance}`];
  return [valueWord(entry.value ?? ""), unit && /^-?\d/.test(entry.value ?? "") ? unit : ""];
}

/** The value held, the unit apart so it can be quieter. */
function heldValue(state, entry) {
  if (!entry.priority) return "";
  const [value, unit] = writtenValue(state, entry);
  return `${h(value)}${unit ? `<span class="unit">${h(unit)}</span>` : ""}`;
}

function heldRows(state) {
  if (!state.held.length) return `<div class="empty"><h2>${t("held.noneTitle")}</h2><span>${t("held.noneText")}</span></div>`;
  return state.held.map((entry, index) => {
    const place = [pointPlace(state, entry), t("held.since", { time: heldSince(entry) })].filter(Boolean).join(" · ");
    const chip = entry.priority ? `${t("priority")} ${entry.priority}` : t("chip.outOfService");
    return `<div class="row held"><span class="name">${h(pointName(state, entry))}<span class="desc">${h(place)}</span></span>
      <span class="val">${heldValue(state, entry)}</span><span class="chips"><span class="chip warn">${h(chip)}</span></span>
      <span class="act"><button type="button" class="quiet" data-act="release-held" data-index="${index}">${t("release")}</button></span></div>`;
  }).join("");
}

/** What a log entry did, in words: "set to 50 % at priority 8". */
function logWhat(state, entry) {
  const actions = { write: () => t("log.wrote", { value: writtenValue(state, entry).filter(Boolean).join(" ") }), release: () => t("log.released"), clock: () => t("log.clock", { value: entry.value }), property: () => t("log.property", { value: entry.value }), command: () => trunkText(entry.value ?? "") };
  const what = actions[entry.action]?.() ?? entry.action;
  return entry.priority ? what + t("log.atPriority", { priority: entry.priority }) : what;
}

/** Newest first; only a refused write gets a chip, with the device's reason beside it. */
function logRows(state) {
  if (!state.log.length) return `<div class="empty"><span>${t("log.empty")}</span></div>`;
  return [...state.log].reverse().map((entry) => {
    const time = new Date(entry.time * 1000).toLocaleTimeString(currentLocale());
    const refused = entry.error ? `<span class="chip err">${t("log.refusedShort")}</span>` : "";
    const reason = entry.error ? `<span class="desc reason" title="${h(entry.error)}">${h(entry.error)}</span>` : "<span></span>";
    return `<div class="row log${entry.error ? " refused" : ""}"><span class="when">${h(time)}</span><span class="name">${h(pointName(state, entry))}<span class="desc">${h([pointPlace(state, entry), logWhat(state, entry)].filter(Boolean).join(" · "))}</span></span>
      <span class="chips">${refused}</span>${reason}</div>`;
  }).join("");
}

/** Values held stay in the plant after you have gone, so leaving asks first. */
function leaveHtml(state) {
  if (!state.leaving) return "";
  const list = state.held.map((entry) => `<li>${h(entryLabel(state, entry))}${entry.priority ? ` = ${h(valueWord(entry.value))} · ${t("priority")} ${entry.priority}` : ` · ${t("chip.outOfService")}`}</li>`).join("");
  return `<div class="modal" role="dialog" aria-modal="true"><div class="modal-box"><h2>${state.held.length === 1 ? t("leave.titleOne") : t("leave.title", { n: state.held.length })}</h2>
    <ul>${list}</ul><p>${t("leave.text")}</p>
    <div class="actions"><button type="button" data-act="leave-release">${t("leave.releaseAll")}</button><button type="button" class="quiet" data-act="leave-stay">${t("leave.stay")}</button></div></div></div>`;
}

/** The button that releases what is held, naming how many: here and beside the workspace's next step. */
export const releaseLabel = (count) => (count === 1 ? t("next.releaseOne") : t("next.release", { n: count }));

/** How many values are held and what to do about them; the next step names how many it releases. */
function heldLead(state) {
  const count = state.held.length;
  if (!count) return { lead: t("held.leadNone"), next: null };
  return { lead: count === 1 ? t("held.leadOne") : t("held.lead", { n: count }), next: { act: "release-all", label: releaseLabel(count) } };
}

export function viewLog(state) {
  const { lead, next } = heldLead(state);
  const offset = -new Date().getTimezoneOffset();
  return {
    title: t("tab.log"),
    lead,
    next,
    html: `<div class="split single"><section class="listwrap"><div class="toolbar"><div class="end">
      <a class="quiet" href="/api/log.csv?offset_min=${offset}" download>${t("log.export")}</a>
      <button type="button" class="quiet" data-act="leave">${t("leave")}</button></div></div><div class="rows" data-keep="log">
      <div class="row group plain"><span class="name">${t("held.now")}</span><span class="count">${state.held.length}</span></div>
      ${heldRows(state)}
      <div class="row group plain"><span class="name">${t("log.written")}</span><span class="count">${state.log.length}</span></div>
      ${logRows(state)}</div></section></div>${leaveHtml(state)}`,
  };
}
