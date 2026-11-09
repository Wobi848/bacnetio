// The chosen point: reading its details, keeping the value live, trend logs, writing, the clock.

import { api } from "./api.js";
import { TREND_TYPES } from "./format.js";
import { t, valueWord } from "./i18n.js";
import { localClock } from "./device-actions.js";
import { loadSitePoints } from "./site-actions.js";
import { refreshHeld, render, state, toast } from "./store.js";

const OBJECT_POLL_MS = 20000;
const VALUE_POLL_MS = 2000;
const PROPERTY_POLL_MS = 10000;
const OUT_OF_SERVICE_PROPERTY = 81;
const SCHEDULE_TYPE = 17;
const CALENDAR_TYPE = 6;

const showing = (type, instance) => state.area === "workspace" && state.object?.type === type && state.object?.instance === instance;
// a redraw would throw away what the user is choosing in a list field
const choosing = () => document.activeElement?.matches?.("select");

export async function selectPoint(key) {
  if (state.selectedPoint === key && state.object) {
    state.detailOpen = true;
    render();
    return;
  }
  const [type, instance] = key.split(":").map(Number);
  state.selectedPoint = key;
  state.object = null;
  state.objectError = null;
  state.asking = false;
  state.newValue = null;
  state.confirm = null;
  state.technical = false;
  state.editProperty = null;
  state.changedProperties = {};
  state.properties = null;
  state.liveSource = null;
  state.trend = null;
  state.times = null;
  state.editDay = null;
  state.error = null;
  state.detailOpen = true;
  render();
  try {
    const object = await api.object(state.device.instance, type, instance);
    if (state.selectedPoint !== key) return;
    state.object = object;
  } catch (error) {
    if (state.selectedPoint === key) state.objectError = error.message;
    render();
    return;
  }
  render();
  if (TREND_TYPES.includes(type)) {
    loadTrend(type, instance);
    return;
  }
  if (type === SCHEDULE_TYPE || type === CALENDAR_TYPE) loadTimes(type, instance);
  setTimeout(() => refreshObject(type, instance), OBJECT_POLL_MS);
  pollValue(type, instance);
}

/** Types what the day holds now into the field, for changing. */
export function editDay(day) {
  state.editDay = day;
  state.editDayText = state.times.week[day].map((change) => change.exact).join("; ");
  render();
  document.getElementById("day-changes")?.focus();
}

export async function writeDay() {
  const { type, instance } = state.object;
  try {
    await api.writeDay(state.device.instance, type, instance, state.editDay, state.editDayText);
    state.editDay = null;
    toast(t("sched.written"));
    await loadTimes(type, instance);
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** A schedule's week and exceptions, a calendar's dates: once, as they rarely change. */
async function loadTimes(type, instance) {
  try {
    state.times = type === SCHEDULE_TYPE
      ? await api.schedule(state.device.instance, type, instance)
      : { dates: await api.calendar(state.device.instance, type, instance) };
  } catch (error) {
    state.times = { error: error.message };
  }
  if (showing(type, instance)) render();
}

async function loadTrend(type, instance) {
  try {
    state.trend = await api.trend(state.device.instance, type, instance);
  } catch (error) {
    state.trend = { error: error.message };
  }
  if (showing(type, instance)) render();
}

/** The present value every 2 s: from the device's own reports where it sends them (COV). */
async function pollValue(type, instance) {
  if (!showing(type, instance)) return;
  try {
    const live = await api.value(state.device.instance, type, instance, state.liveModes[`${state.device.instance}:${type}:${instance}`]);
    if (showing(type, instance)) {
      const units = state.object.units;
      const before = state.object.value;
      state.object.value = units && /^-?\d/.test(live.value) ? `${live.value} ${units}` : live.value;
      // a new value mostly means someone wrote a priority: show who, not only what
      if (before !== state.object.value && state.object.priority_array) readObjectAgain(type, instance);
      if (live.status !== null) state.object.status = live.status;
      state.liveSource = live.source;
      const listed = state.points?.find((point) => point.type === type && point.instance === instance);
      if (listed) listed.value = state.object.value;
      if (!choosing() && !state.onChart) render();
    }
  } catch {
    // the next round tries again; a device that stays silent shows its last value
  }
  setTimeout(() => pollValue(type, instance), VALUE_POLL_MS);
}

/** Reads the whole object now and then, for changes in the priority array by others. */
async function refreshObject(type, instance) {
  if (!showing(type, instance)) return;
  await readObjectAgain(type, instance);
  setTimeout(() => refreshObject(type, instance), OBJECT_POLL_MS);
}

async function readObjectAgain(type, instance) {
  try {
    const fresh = await api.object(state.device.instance, type, instance);
    // the server drops what another system wrote over from the held values; the list follows
    const device = state.device.instance;
    if (state.held.some((entry) => entry.device === device && entry.type === type && entry.instance === instance)) await refreshHeld();
    if (showing(type, instance) && !choosing()) {
      state.object = fresh;
      render();
    }
  } catch {
    // the next round tries again; a device that stays silent shows its last value
  }
}

/** Runs a write or release that answers with the object read again. */
async function change(request, done) {
  state.error = null;
  try {
    state.object = await request();
    // the list must not keep showing the value from before the change
    const listed = state.points?.find((point) => point.type === state.object.type && point.instance === state.object.instance);
    if (listed) listed.value = state.object.value;
    toast(done);
  } catch (error) {
    state.error = error.message;
  }
  await refreshHeld();
  render();
  loadSitePoints();
}

/** The priority that holds the value now, when it is not `priority` itself and is higher. */
function higherThan(priority) {
  const winner = (state.object.priority_array ?? []).find((slot) => slot.value !== null);
  return winner && winner.priority < priority ? winner : null;
}

export function writeValue(anyway = false) {
  const { type, instance } = state.object;
  const value = anyway ? state.confirm.value : document.getElementById("new-value").value;
  const choice = document.getElementById("priority");
  const priority = anyway ? state.confirm.priority : choice ? Number(choice.value) : null;
  const winner = priority && higherThan(priority);
  if (winner && !anyway) {
    state.confirm = { kind: "write", value, priority, winner };
    render();
    return;
  }
  state.confirm = null;
  state.newValue = null;
  const done = priority ? t("wrote", { value: valueWord(value), priority }) : t("wroteDirect", { value: valueWord(value) });
  change(() => api.write(state.device.instance, type, instance, value, priority), done);
}

export function releaseHere(priority, anyway = false) {
  const { type, instance } = state.object;
  const winner = higherThan(priority);
  if (winner && !anyway) {
    state.confirm = { kind: "release", priority, winner };
    render();
    return;
  }
  state.confirm = null;
  change(() => api.release(state.device.instance, type, instance, priority), t("released", { priority }));
}

/** A value bacnetio did not write: released only after a clear question. */
export function releaseOther(priority) {
  const winner = state.object.priority_array.find((slot) => slot.priority === priority);
  state.confirm = { kind: "releaseOther", priority, winner };
  render();
}

export async function releaseHeld(entries) {
  let released = 0;
  for (const entry of entries) {
    try {
      // the device answers nothing but this, so the server does not ask where it is first
      if (entry.stops_communication) await api.communication(entry.device, true, null, null);
      else if (entry.action === "property") await api.writeProperty(entry.device, entry.type, entry.instance, { property_id: OUT_OF_SERVICE_PROPERTY, data_type: "boolean", value: "false" });
      // days may have passed: the server releases only while the slot still holds our value
      else await api.release(entry.device, entry.type, entry.instance, entry.priority, true);
      released += 1;
    } catch (error) {
      toast(`${entry.object}: ${error.message}`, true);
    }
  }
  await refreshHeld();
  state.log = await api.log();
  // the open object shows the release; with nothing released there is nothing new to show, and
  // after a restart there may be no connection to read it over
  if (released && state.object && state.device) {
    try {
      state.object = await api.object(state.device.instance, state.object.type, state.object.instance);
    } catch (error) {
      toast(error.message, true);
    }
  }
  render();
}

export async function showTechnical() {
  state.technical = !state.technical;
  render();
  if (!state.technical || state.properties !== null) return;
  const { type, instance } = state.object;
  try {
    state.properties = await api.properties(state.device.instance, type, instance);
  } catch (error) {
    state.properties = [{ property: "error", value: null, error: error.message }];
  }
  render();
  setTimeout(() => refreshProperties(type, instance), PROPERTY_POLL_MS);
}

/** All properties again and again while they are shown, marking those that changed. */
async function refreshProperties(type, instance) {
  if (!showing(type, instance) || !state.technical) return;
  try {
    const fresh = await api.properties(state.device.instance, type, instance);
    if (showing(type, instance) && state.technical) {
      const before = new Map((state.properties ?? []).map((item) => [item.property, item.value]));
      for (const item of fresh) {
        if (before.has(item.property) && before.get(item.property) !== item.value) state.changedProperties[item.property] = true;
      }
      state.properties = fresh;
      if (!choosing() && !state.onChart) render();
    }
  } catch {
    // the next round tries again; a device that stays silent shows what it said last
  }
  setTimeout(() => refreshProperties(type, instance), PROPERTY_POLL_MS);
}

export function editProperty(property) {
  state.editProperty = property;
  state.editValue = state.properties.find((item) => item.property === property)?.value ?? "";
  render();
  document.getElementById("property-value")?.focus();
}

export async function writeProperty() {
  const { type, instance } = state.object;
  const { editProperty: property, editValue: value } = state;
  const item = state.properties.find((found) => found.property === property);
  state.editProperty = null;
  await change(() => api.writeProperty(state.device.instance, type, instance, { property_id: item.id, data_type: item.data_type, value }), t("property.written", { property, value }));
  // a new name or description must be found by the list's search at once
  const listed = state.points?.find((point) => point.type === type && point.instance === instance);
  if (listed) Object.assign(listed, { name: state.object.name, description: state.object.description });
  try {
    state.properties = await api.properties(state.device.instance, type, instance);
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** Sends this computer's local time to the chosen device. */
export async function setClock() {
  try {
    const clock = localClock();
    state.asking = false;
    await api.setClock(state.selectedDevice, clock);
    toast(t("clock.done", { time: clock.time }));
  } catch (error) {
    toast(error.message, true);
  }
  render();
}
