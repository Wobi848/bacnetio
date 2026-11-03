// The log: values you still hold in the plant, with release, and every write of this session.

import { DEVICE_TYPE, deviceName } from "./format.js";
import { h } from "./html.js";
import { currentLocale, objectText, t, valueWord } from "./i18n.js";

/** "AHU-01 · Heating valve" for a held or logged entry, with the names known from reading. */
export function entryLabel(state, entry) {
  const device = state.scan.devices?.find((found) => found.instance === entry.device);
  const owner = device ? deviceName(device) : t("device", { n: entry.device });
  // the clock belongs to the device itself: its name once is enough
  if (entry.type === DEVICE_TYPE) return owner;
  const name = state.objectNames[`${entry.device}:${entry.type}:${entry.instance}`];
  return `${owner} · ${name ?? objectText(entry)}`;
}

function heldRows(state) {
  if (!state.held.length) return `<div class="empty"><p>${t("nothingHeld")}</p></div>`;
  return state.held.map((entry, index) => `<div class="row log"><span class="mono">${entry.priority ? `${t("priority")} ${entry.priority}` : t("chip.outOfService")}</span>
    <span class="name">${h(entryLabel(state, entry))}</span><span class="mono">${h(entry.priority ? valueWord(entry.value) : "")}</span>
    <button type="button" class="quiet" data-act="release-held" data-index="${index}">${t("release")}</button></div>`).join("");
}

function logRows(state) {
  if (!state.log.length) return `<div class="empty"><p>${t("log.empty")}</p></div>`;
  return [...state.log].reverse().map((entry) => {
    const time = new Date(entry.time * 1000).toLocaleTimeString(currentLocale());
    const actions = { write: () => t("log.wrote", { value: valueWord(entry.value) }), release: () => t("log.released"), clock: () => t("log.clock", { value: entry.value }), property: () => t("log.property", { value: entry.value }), command: () => entry.value ?? "" };
    let what = actions[entry.action]?.() ?? entry.action;
    if (entry.priority) what += t("log.atPriority", { priority: entry.priority });
    const result = entry.error ? `<span class="chip err" title="${h(entry.error)}">${t("log.refusedShort")}</span>` : `<span class="chip ok">${t("log.ok")}</span>`;
    return `<div class="row log"><span class="mono">${h(time)}</span><span class="name">${h(entryLabel(state, entry))}</span><span class="cut">${h(what)}</span>${result}</div>`;
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

export function viewLog(state) {
  return {
    title: t("tab.log"),
    lead: t("log.lead"),
    next: state.held.length ? { act: "release-all", label: t("releaseAll") } : null,
    html: `<div class="split single"><section class="listwrap"><div class="toolbar"><div class="end"><button type="button" class="quiet" data-act="leave">${t("leave")}</button></div></div><div class="rows" data-keep="log">
      <div class="row group plain"><span class="name">${t("held.title")}</span><span class="count">${state.held.length}</span></div>
      ${heldRows(state)}
      <div class="row group plain"><span class="name">${t("changeLog")}</span><span class="count">${state.log.length}</span></div>
      ${logRows(state)}</div></section></div>${leaveHtml(state)}`,
  };
}
