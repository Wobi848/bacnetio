// Choosing points in the list and changing them all at once (change-many.js): each write is a
// single write of the server, checked, logged and counted as held there.

import { api } from "./api.js";
import { CHANGES, MOST_POINTS, plan, toWrite } from "./change-many.js";
import { DEVICE_TYPE, pointKey } from "./format.js";
import { t } from "./i18n.js";
import { shownGroups, visiblePoints } from "./points.js";
import { refreshHeld, render, state, toast } from "./store.js";

export function toggleSelecting() {
  state.selecting = !state.selecting;
  if (!state.selecting) state.selected = [];
  render();
}

export function togglePicked(key) {
  state.selected = state.selected.includes(key) ? state.selected.filter((picked) => picked !== key) : [...state.selected, key];
  render();
}

/** One change takes at most so many points: the rest of a choice is given back, with a word. */
function keepToMost() {
  if (state.selected.length <= MOST_POINTS) return;
  state.selected = state.selected.slice(0, MOST_POINTS);
  toast(t("many.most", { n: MOST_POINTS }), true);
}

/** Chooses the points the filter and the view show, up to the most one change takes. */
export function pickShown() {
  const keys = visiblePoints(state).map(pointKey);
  state.selected = [...new Set([...state.selected, ...keys])];
  keepToMost();
  render();
}

/** A group's box: all of its points shown, or none of them when all were chosen. */
export function pickGroup(groupKey) {
  const group = shownGroups(state, devicePoints(), visiblePoints(state)).find((found) => found.key === groupKey);
  if (!group) return;
  const keys = group.items.map(({ point }) => pointKey(point));
  const all = keys.every((key) => state.selected.includes(key));
  state.selected = all ? state.selected.filter((key) => !keys.includes(key)) : [...new Set([...state.selected, ...keys])];
  keepToMost();
  render();
}

export function pickNone() {
  state.selected = [];
  render();
}

export function openChangeMany() {
  state.changeMany = { kind: "value", value: "", priority: state.writePriority ?? 8, phase: "edit", results: {}, stopping: false };
  render();
}

const devicePoints = () => (state.points ?? []).filter((point) => point.type !== DEVICE_TYPE);

/** One point's write, as the details would make it. */
function writeOne(device, point, many) {
  const change = CHANGES[many.kind];
  if (many.kind === "value") return api.write(device, point.type, point.instance, many.value.trim(), many.priority);
  if (many.kind === "release") return api.release(device, point.type, point.instance, many.priority);
  return api.writeProperty(device, point.type, point.instance, { property_id: change.property, data_type: change.dataType, value: many.value });
}

/** Writes one point after the other; Stop ends after the point being written. */
export async function writeMany() {
  const many = state.changeMany;
  const device = state.device.instance;
  const chosen = devicePoints().filter((point) => state.selected.includes(pointKey(point)));
  const targets = toWrite(plan(many.kind, chosen, many.value)).map((row) => row.point);
  many.phase = "writing";
  for (const point of targets) {
    if (many.stopping || state.changeMany !== many) break;
    const key = pointKey(point);
    many.results[key] = { state: "writing" };
    render();
    try {
      await writeOne(device, point, many);
      many.results[key] = { state: "done" };
    } catch (error) {
      many.results[key] = { state: "failed", reason: error.message };
    }
  }
  many.phase = "done";
  render();
  await refreshAfter(device, targets);
}

/** What the change left: the values in the list, what you hold, the log. */
async function refreshAfter(device, points) {
  try {
    await refreshHeld();
    state.log = await api.log();
    const fresh = await api.liveValues(device, points.map(({ type, instance }) => ({ type, instance })));
    const byKey = new Map(devicePoints().map((point) => [pointKey(point), point]));
    for (const point of fresh) {
      const known = byKey.get(pointKey(point));
      if (known) Object.assign(known, { value: point.value, status: point.status, error: point.error });
    }
  } catch (error) {
    toast(error.message, true);
  }
  render();
}
