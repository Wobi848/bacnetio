// The properties of an object in the details (docs/concept-properties.md): in sections a
// technician knows, named in the language of the interface, values by their meaning. A switch
// shows the BACnet names and what the device sent, to compare with the standard or another tool.

import { h } from "./html.js";
import { eventStateText, objectText, objectTypeName, propertyValueText, t, valueWord } from "./i18n.js";

/** The sections and their properties, in the order shown; the rest goes under *More*. */
const SECTIONS = [
  ["props.object", ["object-name", "description", "object-identifier", "object-type", "profile-name"]],
  ["props.value", ["present-value", "status-flags", "reliability", "out-of-service", "units", "cov-increment", "relinquish-default",
    "polarity", "inactive-text", "active-text", "number-of-states", "state-text", "min-pres-value", "max-pres-value", "resolution"]],
  ["props.limits", ["high-limit", "low-limit", "deadband", "limit-enable", "time-delay", "time-delay-normal", "alarm-value", "alarm-values", "feedback-value"]],
  ["props.alarming", ["event-state", "event-detection-enable", "event-enable", "acked-transitions", "notify-type", "notification-class",
    "event-time-stamps", "event-message-texts", "event-message-texts-config"]],
];
const SECTIONED = new Set(SECTIONS.flatMap(([, names]) => names));

/** The three transitions of an alarm, in the order BACnet lists them in bits and arrays. */
const TRANSITIONS = ["props.toAlarm", "props.toFault", "props.toNormal"];
/** What each bit of a bit string means, first bit first. */
const BIT_NAMES = {
  "status-flags": ["chip.alarm", "chip.fault", "chip.overridden", "chip.outOfService"],
  "event-enable": TRANSITIONS,
  "acked-transitions": TRANSITIONS,
  "limit-enable": ["prop.low-limit", "prop.high-limit"],
};
/** Lists with one entry per transition. */
const PER_TRANSITION = ["event-time-stamps", "event-message-texts", "event-message-texts-config"];
/** Which bit of limit-enable switches each limit on. */
const LIMIT_BIT = { "low-limit": 0, "high-limit": 1 };
/** Plain numbers that BACnet counts in seconds. */
const SECONDS = ["time-delay", "time-delay-normal"];
const NOTIFY_TYPES = { alarm: "chip.alarm", event: "props.event" };
/** The server's word for a value it cannot put into words yet; the BACnet view shows it as sent. */
const STRUCTURED = "(structured value)";

/** "event-detection-enable" → "Event detection enable", where no language names it. */
const sentence = (name) => name.charAt(0).toUpperCase() + name.slice(1).replaceAll("-", " ");

export function propertyLabel(name) {
  const text = t(`prop.${name}`);
  return text === `prop.${name}` ? sentence(name) : text;
}

const chip = (kind, text) => `<span class="chip ${kind}">${h(text)}</span>`;
const empty = (text) => `<span class="blank">${h(text)}</span>`;
const setNames = (item) => (item.bits ?? []).flatMap((set, index) => (set && BIT_NAMES[item.property][index] ? [t(BIT_NAMES[item.property][index])] : []));

function bitsHtml(item) {
  const names = setNames(item);
  switch (item.property) {
    case "status-flags":
      if (!names.length) return chip("ok", t("normal"));
      return item.bits.map((set, index) => (set ? chip(index < 2 ? "err" : "warn", t(BIT_NAMES["status-flags"][index])) : "")).join("");
    case "acked-transitions": {
      const open = item.bits.flatMap((set, index) => (set ? [] : [t(TRANSITIONS[index])]));
      return open.length ? chip("warn", t("props.notAcked", { what: open.join(", ") })) : chip("ok", t("props.allAcked"));
    }
    case "event-enable":
      return names.length ? h(names.join(", ")) : empty(t("props.nothingOn"));
    default:
      return names.length ? h(names.join(", ")) : empty(t("props.none"));
  }
}

/** A date and time from the server ("2026-11-02 14:03:00") the way the interface writes them. */
function stampText(text) {
  if (text === "never") return empty(t("props.never"));
  return h(text);
}

function itemsHtml(item) {
  const entries = item.items;
  if (!entries.length || entries.every((entry) => entry === "")) return empty(t("props.empty"));
  if (PER_TRANSITION.includes(item.property)) {
    const rows = entries.map((entry, index) => `<span>${TRANSITIONS[index] ? t(TRANSITIONS[index]) : index + 1}</span><span>${item.property === "event-time-stamps" ? stampText(entry) : entry ? h(entry) : empty(t("props.empty"))}</span>`);
    return `<span class="stamps">${rows.join("")}</span>`;
  }
  if (entries.every((entry) => entry === STRUCTURED)) return empty(t("props.structured"));
  return `<span class="entries">${entries.map((entry) => `<span>${entry ? h(valueWord(entry)) : empty(t("props.empty"))}</span>`).join("")}</span>`;
}

/** The value in the plain view: by what it means. */
function plainHtml(item, all, object) {
  const value = item.value ?? "";
  if (item.bits && BIT_NAMES[item.property]) return bitsHtml(item);
  if (item.items) return itemsHtml(item);
  if (item.data_type === "boolean") return h(t(value === "true" ? "props.yes" : "props.no"));
  if (item.data_type === "text" && value === "") return empty(t("props.empty"));
  if (value === STRUCTURED) return empty(t("props.structured"));
  switch (item.property) {
    case "object-identifier": return h(objectText(object));
    case "object-type": return h(objectTypeName(object.type));
    case "event-state":
      if (value === "normal") return chip("ok", t("normal"));
      return chip(value === "fault" ? "err" : "warn", eventStateText(value));
    case "reliability":
      return value === "no fault detected" ? chip("ok", t("props.noFault")) : chip("err", value);
    case "notify-type":
      return h(NOTIFY_TYPES[value] ? t(NOTIFY_TYPES[value]) : value);
  }
  if (SECONDS.includes(item.property)) return `${h(value)} <span class="unit">s</span>`;
  const text = h(valueWord(propertyValueText(value)));
  if (item.property in LIMIT_BIT) {
    const enable = all.find((other) => other.property === "limit-enable");
    if (enable?.bits && !enable.bits[LIMIT_BIT[item.property]]) return `${text} <span class="unit">· ${t("props.notMonitored")}</span>`;
  }
  return text;
}

function rowHtml(state, item, all, object) {
  const bacnet = state.propertyNames === "bacnet";
  const names = bacnet ? `<span class="bn">${h(item.property)}${item.data_type ? ` · ${h(item.data_type)}` : ""}</span>` : "";
  const label = `<dt>${h(propertyLabel(item.property))}${names}</dt>`;
  if (state.editProperty === item.property) {
    return `${label}<dd><div class="property-edit"><input id="property-value" class="mono" value="${h(state.editValue)}">
      <button type="button" data-act="property-write">${t("writeValue")}</button><button type="button" class="quiet" data-act="property-cancel">${t("cancel")}</button></div></dd>`;
  }
  const changed = state.changedProperties[item.property] ? ' class="changed"' : "";
  let value;
  if (item.error) value = chip("warn", item.error);
  else if (bacnet) value = item.property === "object-identifier" ? h(`type ${object.type}, instance ${object.instance}`) : `<span class="mono">${h(propertyValueText(item.value))}</span>`;
  else value = plainHtml(item, all, object);
  const change = item.writable && state.writeUnlocked && !item.error ? `<button type="button" class="quiet" data-act="property-edit" data-property="${h(item.property)}">${t("change")}</button>` : "";
  return `${label}<dd${changed}>${value}${change}</dd>`;
}

const label = (name) => (name === "device" ? t("props.device") : propertyLabel(name));
const matches = (item, filter) => !filter || `${item.property} ${label(item.property)}`.toLowerCase().includes(filter);

/** The properties of `object`, read from `state.properties`, in their sections. */
export function propertySectionsHtml(state, object) {
  const filter = state.propertyFilter.trim().toLowerCase();
  const all = state.properties;
  const shown = all.filter((item) => matches(item, filter));
  const rows = (items, extra = "") => `<dl class="props">${items.map((item) => rowHtml(state, item, all, object)).join("")}${extra}</dl>`;
  // the device is no property of the object, but belongs with what names it
  const device = matches({ property: "device" }, filter)
    ? `<dt>${t("props.device")}</dt><dd>${state.device.instance} <span class="unit">· ${h(state.device.address)}</span></dd>` : "";
  const sections = SECTIONS.map(([title, names], index) => {
    const items = names.flatMap((name) => shown.filter((item) => item.property === name));
    const extra = index === 0 ? device : "";
    return items.length || extra ? `<h3>${t(title)}</h3>${rows(items, extra)}` : "";
  }).join("");
  const others = shown.filter((item) => !SECTIONED.has(item.property));
  const othersOpen = state.propertiesOther || Boolean(filter);
  const more = others.length ? `<button type="button" class="section-toggle" data-act="properties-other" aria-expanded="${othersOpen}">${t("props.other", { n: others.length })} <span aria-hidden="true">${othersOpen ? "▾" : "▸"}</span></button>${othersOpen ? rows(others) : ""}` : "";
  if (!sections && !more) return `<p class="hint">${t("props.noMatch")}</p>`;
  return sections + more;
}

/** Search and the switch between plain words and BACnet names. */
export function propertiesBarHtml(state) {
  const names = [["plain", "props.plain"], ["bacnet", "props.bacnet"]].map(([name, label]) =>
    `<button type="button" data-act="property-names" data-names="${name}" aria-pressed="${state.propertyNames === name}">${t(label)}</button>`).join("");
  return `<div class="props-bar"><input id="property-filter" type="search" placeholder="${t("props.search")}" value="${h(state.propertyFilter)}" autocomplete="off">
    <span class="seg" title="${t("props.namesHint")}">${names}</span></div>`;
}
