// The chosen data point beside the list: its value, who sets it, writing, trend logs and all
// properties.

import { figures } from "./analysis.js";
import { chartHtml, numberText } from "./chart.js";
import { MULTISTATE, RECORDABLE, TREND_LOG_TYPE, chips, errorStrip, has, isBinary, split, stateWord, valueHtml } from "./format.js";
import { h } from "./html.js";
import { currentLocale, objectText, statusText, t, valueWord } from "./i18n.js";
import { priorityLabel } from "./plugins.js";
import { propertiesBarHtml, propertySectionsHtml } from "./properties.js";

/** Schedule and calendar objects. */
const TIME_TYPES = [17, 6];
const PRIORITIES = Array.from({ length: 16 }, (_, index) => index + 1);
/** Objects whose present value is a number or a state, the only ones a recording can chart. */
/** Value objects may be writable without a priority array; the device decides. */
const WRITABLE_WITHOUT_PRIORITY = [2, 5, 19, 40, 45, 46, 48, 54, 55];

/** The point as read with the list: its state texts name multi-state values. */
const listed = (state, object) => state.points?.find((point) => point.type === object.type && point.instance === object.instance);

/** All 16 priorities at once: who holds what, what wins now. The lowest number wins. */
function slotsHtml(state, object, stateTexts) {
  const slots = object.priority_array ?? [];
  const winner = slots.find((slot) => slot.value !== null);
  const cells = slots.map((slot) => {
    const held = slot.value !== null;
    const classes = ["slot", held ? "held" : "", slot === winner ? "win" : "", slot.mine ? "mine" : "", state.writeUnlocked && state.writePriority === slot.priority ? "chosen" : ""].filter(Boolean).join(" ");
    const label = priorityLabel(state, slot.priority);
    const title = label.name ? ` title="${h(label.by ? `${label.name} (${label.by})` : label.name)}"` : "";
    return `<li class="${classes}"${title}${state.writeUnlocked ? ` data-act="choose-priority" data-priority="${slot.priority}" role="button"` : ""}>
      <span class="n">${slot.priority}</span><span class="v">${held ? h(stateWord(slot.value, stateTexts)) : "–"}</span>
      <span class="who">${h(slot.mine ? t("you") : label.name)}</span></li>`;
  }).join("");
  const fallback = object.relinquish_default === null ? "?" : stateWord(object.relinquish_default, stateTexts);
  const sentence = winner
    ? t("stack.wins", { priority: winner.priority, value: `<span class="value-word">${h(stateWord(winner.value, stateTexts))}</span>` })
    : t("stack.free", { value: `<span class="value-word">${h(fallback)}</span>` });
  return `<ul class="slots">${cells}</ul><p class="stack-now">${sentence}</p>${winner ? `<p class="hint">${t("default")}: ${h(fallback)}</p>` : ""}`;
}

function inputHtml(state, object, stateTexts) {
  const draft = state.newValue;
  if (isBinary(object)) {
    const option = (value, label) => `<option value="${value}"${draft === value ? " selected" : ""}>${label}</option>`;
    return `<label>${t("newValue")}<select id="new-value">${option("active", t("active"))}${option("inactive", t("inactive"))}</select></label>`;
  }
  if (MULTISTATE.includes(object.type) && stateTexts?.length) {
    const current = draft ?? /^state (\d+)$/.exec(object.value)?.[1];
    const options = stateTexts.map((text, index) => `<option value="${index + 1}"${String(index + 1) === current ? " selected" : ""}>${index + 1} · ${h(text || t("state", { n: index + 1 }))}</option>`).join("");
    return `<label>${t("newState")}<select id="new-value">${options}</select></label>`;
  }
  if (MULTISTATE.includes(object.type)) {
    return `<label>${t("newState")}<input id="new-value" class="mono" inputmode="numeric" value="${h(draft ?? object.value.replace(/^state /, ""))}"></label>`;
  }
  return `<label>${t("newValue")}${object.units ? ` (${h(object.units)})` : ""}<input id="new-value" class="mono" inputmode="decimal" value="${h(draft ?? valueWord(split(object)[0]))}"></label>`;
}

/** A write or release that would not act asks first. */
function confirmHtml(state, stateTexts) {
  const { kind, priority, winner } = state.confirm;
  const values = { priority, winner: winner.priority, value: h(stateWord(winner.value, stateTexts)) };
  return `<div class="ask"><b>${t(`confirm.${kind}`, values)}</b>
    <div class="actions"><button type="button" data-act="confirm-yes">${t(`confirm.${kind}Anyway`, values)}</button><button type="button" class="quiet" data-act="confirm-no">${t("cancel")}</button></div></div>`;
}

function writeHtml(state, object, stateTexts) {
  const mine = (object.priority_array ?? []).filter((slot) => slot.mine);
  const release = mine.map((slot) => `<button type="button" class="tonal" data-act="release" data-priority="${slot.priority}">${t("releaseMine", { priority: slot.priority })}</button>`).join("");
  if (!state.writeUnlocked && state.asking !== true) {
    return `<div class="actions"><button type="button" class="tonal" data-act="ask-write">${t("changeValue")}</button>${release}</div>`;
  }
  if (!state.writeUnlocked) {
    return `<div class="ask"><b>${t("unlock.title")}</b><span>${t("unlock.text")}</span>
      <div class="actions"><button type="button" data-act="unlock">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
  }
  const priorities = PRIORITIES.map((priority) => `<option value="${priority}"${priority === state.writePriority ? " selected" : ""}>${[priority, priorityLabel(state, priority).name].filter(Boolean).join(" · ")}</option>`).join("");
  const priority = object.commandable ? `<label>${t("priority")}<select id="priority">${priorities}</select></label>` : "";
  if (state.confirm) return confirmHtml(state, stateTexts);
  // someone else's value, chosen in the grid: the plant may depend on it, hence the question
  const chosen = (object.priority_array ?? []).find((slot) => slot.priority === state.writePriority);
  const releaseOther = object.commandable && chosen && chosen.value !== null && !chosen.mine
    ? `<button type="button" class="quiet" data-act="release-other" data-priority="${chosen.priority}">${t("releaseOther", { priority: chosen.priority })}</button>` : "";
  return `<div class="form">${inputHtml(state, object, stateTexts)}${priority}</div>
    <div class="actions"><button type="button" data-act="write">${t("writeValue")}</button>${release}${releaseOther}</div>`;
}

function trendHtml(state, object) {
  const trend = state.trend;
  if (!trend) return `<p class="hint">${t("trend.reading")}</p>`;
  if (trend.error) return errorStrip(trend.error);
  const series = { samples: trend.records.map(([time, value]) => [Date.parse(time), value]), units: "", label: object.name || objectText(object) };
  const notes = [trend.gaps ? t("trend.gaps") : "", trend.truncated ? t("trend.truncated") : ""].filter(Boolean).map((text) => `<p class="hint">${h(text)}</p>`).join("");
  const csv = trend.records.length ? `<div class="actions"><a class="quiet" href="/api/devices/${state.device.instance}/objects/${object.type}/${object.instance}/trend.csv" download>${t("trend.export")}</a></div>` : "";
  const found = figures(series.samples);
  const summary = found.low === null ? "" : `<dl class="facts"><dt>${t("rec.low")}</dt><dd class="mono">${h(numberText(found.low))}</dd><dt>${t("rec.high")}</dt><dd class="mono">${h(numberText(found.high))}</dd><dt>${t("rec.mean")}</dt><dd class="mono">${h(numberText(found.mean))}</dd></dl>`;
  return `<p class="sub">${t("trend.records", { n: trend.records.length })}</p>${chartHtml(`trend-${object.instance}`, series, t("trend.empty"))}${summary}${notes}${csv}`;
}

function changesHtml(changes) {
  return changes.map((change) => `<span class="time-change"><b class="mono">${h(change.time)}</b> ${h(valueWord(change.value))}</span>`).join("") || `<span class="sub">–</span>`;
}

/** The week of a schedule from Monday, and what is different on exceptional days. */
function scheduleHtml(state, times) {
  const day = (index) => new Date(Date.UTC(2024, 0, 1 + index)).toLocaleDateString(currentLocale(), { weekday: "short", timeZone: "UTC" });
  const week = times.week.map((changes, index) => {
    if (state.editDay === index) {
      return `<dt>${h(day(index))}</dt><dd><div class="property-edit"><input id="day-changes" class="mono" value="${h(state.editDayText)}" placeholder="06:00 21.5; 18:00 17">
        <button type="button" data-act="day-write">${t("writeValue")}</button><button type="button" class="quiet" data-act="day-cancel">${t("cancel")}</button></div></dd>`;
    }
    const edit = state.writeUnlocked ? ` <button type="button" class="quiet" data-act="day-edit" data-day="${index}">${t("change")}</button>` : "";
    return `<dt>${h(day(index))}</dt><dd>${changesHtml(changes)}${edit}</dd>`;
  }).join("");
  const exceptions = times.exceptions.map((exception) => `<dt>${h(exception.when)}</dt><dd>${changesHtml(exception.changes)} <span class="sub">${t("priority")} ${exception.priority}</span></dd>`).join("");
  const unlock = state.writeUnlocked ? "" : `<div class="actions"><button type="button" class="tonal" data-act="ask-write">${t("sched.change")}</button></div>${state.asking === true ? `<div class="ask"><b>${t("unlock.title")}</b><span>${t("unlock.text")}</span>
      <div class="actions"><button type="button" data-act="unlock">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>` : ""}`;
  return `<h3>${t("sched.week")}</h3>${unlock}${week ? `<dl class="facts times">${week}</dl>` : `<p class="hint">${t("sched.empty")}</p>`}
    ${exceptions ? `<h3>${t("sched.exceptions")}</h3><dl class="facts times">${exceptions}</dl>` : ""}`;
}

function timesHtml(state) {
  const times = state.times;
  if (!times) return `<p class="hint">${t("properties.reading")}</p>`;
  if (times.error) return errorStrip(times.error);
  if (times.dates) return `<h3>${t("sched.dates")}</h3>${times.dates.length ? `<ul class="dates">${times.dates.map((date) => `<li>${h(date)}</li>`).join("")}</ul>` : `<p class="hint">${t("sched.empty")}</p>`}`;
  return scheduleHtml(state, times);
}

function propertiesHtml(state, object) {
  if (!state.technical) return "";
  if (state.properties === null) return `<p class="hint">${t("properties.reading")}</p>`;
  return `<div class="props-head">${propertiesBarHtml(state)}
    <p class="hint">${t(state.writeUnlocked ? "properties.live" : "properties.liveLocked")}</p>${propertiesUnlockHtml(state)}</div>
    ${propertySectionsHtml(state, object)}`;
}

/** Objects without a value to write, such as inputs, have their own way to the unlock here. */
function propertiesUnlockHtml(state) {
  if (state.writeUnlocked) return "";
  if (state.asking !== "properties") return `<div class="actions"><button type="button" class="tonal" data-act="ask-properties">${t("properties.change")}</button></div>`;
  return `<div class="ask"><b>${t("unlock.title")}</b><span>${t("unlock.text")}</span>
    <div class="actions"><button type="button" data-act="unlock">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
}

export function detailHtml(state) {
  const object = state.object;
  const open = state.detailOpen ? " open" : "";
  const back = `<button type="button" class="quiet back" data-act="close-detail">← ${t("back")}</button>`;
  if (!object) return `<aside class="detail${open}">${back}${errorStrip(state.objectError) || `<p class="hint">${t("pts.reading")}</p>`}</aside>`;
  const point = listed(state, object);
  const stateTexts = point?.state_texts;
  const isTrend = object.type === TREND_LOG_TYPE;
  const held = state.held.some((entry) => entry.device === state.device.instance && entry.type === object.type && entry.instance === object.instance);
  const live = state.liveSource === "cov" ? `<span class="chip ok" title="${h(t("chip.liveHint"))}">${t("chip.live")}</span>` : "";
  const mode = state.liveModes[`${state.device.instance}:${object.type}:${object.instance}`] ?? "auto";
  const modes = isTrend ? "" : `<div class="live-mode"><span class="sub">${t("liveMode.title")}</span><span class="seg">${[["auto", "liveMode.auto"], ["read", "liveMode.read"]].map(([name, label]) => `<button type="button" data-act="live-mode" data-mode="${name}" aria-pressed="${mode === name}">${t(label)}</button>`).join("")}</span></div>`;
  const writable = object.commandable || WRITABLE_WITHOUT_PRIORITY.includes(object.type);
  const recorded = (state.recording?.points ?? []).some((entry) => entry.device === state.device.instance && entry.type === object.type && entry.instance === object.instance);
  const watched = state.watch.some((entry) => entry.device === state.device.instance && entry.type === object.type && entry.instance === object.instance);
  // one row of toggles: each shows whether it is on, the long words stay as tooltips
  const toggle = (act, on, label, tip) => `<button type="button" data-act="${act}" aria-pressed="${on}" title="${tip}">${label}</button>`;
  const watch = RECORDABLE.includes(object.type) ? toggle(watched ? "watch-remove-here" : "watch-add", watched, t("act.watch"), t(watched ? "watch.remove" : "watch.add")) : "";
  const record = RECORDABLE.includes(object.type) ? toggle(recorded ? "rec-remove-here" : "rec-add", recorded, t("act.record"), t(recorded ? "rec.removeHere" : "rec.add")) : "";
  const facts = [[t("col.type"), objectText(object)], [t("description"), object.description || "–"], [t("status"), statusText(object.status)]];
  if (object.units) facts.push([t("unit"), object.units]);
  if (object.error) facts.push([t("readProblem"), object.error]);
  const value = isTrend ? "" : `<div class="big">${valueHtml(object, stateTexts)}</div>`;
  const who = writable && object.commandable ? `<h3>${t("stack.title")}</h3><p class="hint">${t("stack.lead")}</p>${slotsHtml(state, object, stateTexts)}` : "";
  const direct = writable && !object.commandable ? `<h3>${t("direct.title")}</h3><p class="hint">${t("direct.lead")}</p>` : "";
  return `<aside class="detail${open}" data-keep="point-detail">${back}
    <div class="dhead"><b>${h(object.name || objectText(object))}</b><span class="sub">${h(objectText(object))}</span></div>
    ${value}<div class="chips">${chips(object, held)}${live}</div>${has(object, "overridden") ? `<p class="hint">${t("overridden.why")}</p>` : ""}${modes}
    ${errorStrip(state.error)}
    ${isTrend ? `<h3>${t("trend.title")}</h3>${trendHtml(state, object)}` : ""}
    ${TIME_TYPES.includes(object.type) ? timesHtml(state) : ""}
    ${who}${direct}${writable ? writeHtml(state, object, stateTexts) : ""}
    <h3>${t("details")}</h3>
    <dl class="facts">${facts.map(([label, text]) => `<dt>${h(label)}</dt><dd>${h(text)}</dd>`).join("")}</dl>
    ${watch || record ? `<div class="actions"><span class="seg">${watch}${record}</span></div>` : ""}
    <button type="button" class="quiet disclosure" data-act="technical" aria-expanded="${Boolean(state.technical)}" title="${t(state.technical ? "hideTechnical" : "showTechnical")}">${t("act.technical")}<span aria-hidden="true">${state.technical ? "▴" : "▾"}</span></button>
    ${propertiesHtml(state, object)}
  </aside>`;
}
