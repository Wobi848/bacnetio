// Small pieces of text and HTML shared by the pages.

import { h } from "./html.js";
import { noteText, t, valueWord } from "./i18n.js";

// BACnet object type numbers
export const BINARY = [3, 4, 5];
export const MULTISTATE = [13, 14, 19];
export const DEVICE_TYPE = 8;
export const TREND_LOG_TYPE = 20;
const NAMED_PRIORITIES = [1, 2, 5, 6, 8, 16];

/** Vendor and model, without the brand twice: many models already start with it ("Kieback&Peter DDC4000"). */
export function deviceMaker(device) {
  const vendor = device.vendor ?? "";
  const model = device.model ?? "";
  const brand = vendor.split(/\s+/)[0].toLowerCase();
  if (brand && model.toLowerCase().includes(brand)) return model;
  return [vendor, model].filter(Boolean).join(" ");
}

export const has = (point, flag) => (point.status ?? "").includes(flag);
export const isFault = (point) => has(point, "fault") || has(point, "alarm") || Boolean(point.error);
export const isOverridden = (point, held) => has(point, "overridden") || held;
export const isHeldEntry = (entry, device, point) => entry.device === device && entry.type === point.type && entry.instance === point.instance;

/** "1 device", "3 devices" in the current language. */
export const deviceCount = (n) => `${n} ${n === 1 ? t("oneDeviceWord") : t("devicesWord")}`;
export const deviceName = (device) => device.name || t("device", { n: device.instance });
export const priorityName = (priority) => (NAMED_PRIORITIES.includes(priority) ? t(`priority.${priority}`) : "");
export const isBinary = (point) => BINARY.includes(point.type);
export const isWord = (point) => isBinary(point) || MULTISTATE.includes(point.type);
export const pointKey = (point) => `${point.type}:${point.instance}`;

/** The adapter to keep when the list is read again: the same address, else the same adapter with a new address, else the system's choice. */
export function chooseAdapter(adapters, wanted) {
  return adapters.find((adapter) => adapter.ip === wanted?.ip)
    ?? adapters.find((adapter) => adapter.name === wanted?.name)
    ?? adapters.find((adapter) => adapter.default) ?? adapters[0] ?? null;
}

/** Types whose present value is a number or a state: these can be watched and recorded. */
export const RECORDABLE = [0, 1, 2, 3, 4, 5, 13, 14, 19];

const sameObject = (entry, device, point) => entry.device === device && entry.type === point.type && entry.instance === point.instance;
export const isWatched = (state, device, point) => state.watch.some((entry) => sameObject(entry, device, point));
export const isRecorded = (state, device, point) => (state.recording?.points ?? []).some((entry) => sameObject(entry, device, point));

/** "state 2" → the device's own text for state 2, where it has one. */
export function stateWord(text, stateTexts) {
  const state = /^state (\d+)$/.exec(text ?? "");
  const own = state && stateTexts?.[Number(state[1]) - 1];
  return own || valueWord(text);
}

/** The number and the unit apart: "21.4 °C" → ["21.4", "°C"]. */
export function split(point) {
  const unit = point.units;
  const value = point.value ?? "";
  if (unit && value.endsWith(` ${unit}`)) return [value.slice(0, -unit.length - 1), unit];
  return [value, ""];
}

/** A device's objects without the device object itself, as the table and other browsers count them. */
export const ownObjects = (count) => (count ? count - 1 : count);

/** The value as HTML for a list or the detail area: number and unit apart, words translated. */
export function valueHtml(point, stateTexts) {
  if (isWord(point)) return `<span class="word">${h(stateWord(point.value, stateTexts))}</span>`;
  const [number, unit] = split(point);
  return `<span class="num">${h(valueWord(number))}</span><span class="unit" title="${h(unit ?? "")}">${h(unit ?? "")}</span>`;
}

/** State chips: red for faults and alarms, orange for overrides. Never colour alone. */
export function chips(point, held) {
  const out = [];
  if (held) out.push(`<span class="chip warn">${t("chip.held")}</span>`);
  if (has(point, "alarm")) out.push(`<span class="chip err">${t("chip.alarm")}</span>`);
  if (has(point, "fault")) out.push(`<span class="chip err">${t("chip.fault")}</span>`);
  if (has(point, "overridden")) out.push(`<span class="chip warn">${t("chip.overridden")}</span>`);
  if (has(point, "out of service")) out.push(`<span class="chip warn">${t("chip.outOfService")}</span>`);
  if (point.error) out.push(`<span class="chip err" title="${h(point.error)}">${t("readError")}</span>`);
  return out.join("");
}

/** Notes on what bacnetio already worked around: information, not a call to act, so not orange. */
const WORKED_AROUND = ["broadcast_blocked", "broadcast_not_passed", "some_only_direct"];
export const noteStrip = (note) => `<div class="strip${WORKED_AROUND.includes(note.code) ? " info" : ""}">${h(noteText(note))}</div>`;
export const errorStrip = (text) => (text ? `<div class="strip err">${h(text)}</div>` : "");
