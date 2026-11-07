// Comparing the open device with another (compare.js): choosing it, reading both again, opening a
// point of either side.

import { api } from "./api.js";
import { openPointAt } from "./device-actions.js";
import { selectPoint } from "./object-actions.js";
import { loadSitePoints, readDevice } from "./site-actions.js";
import { render, state, toast } from "./store.js";

export function startCompare() {
  state.compare = { other: null, view: "all", reading: false };
  state.menu = null;
  render();
}

/** Compares with that device, reading it first when its points are not known yet. */
export async function chooseCompare(instance) {
  state.compare.other = instance;
  render();
  if (state.sitePoints[instance]) return;
  try {
    if (!(await readDevice(instance))) toast((await api.readProgress(instance)).error, true);
  } catch (error) {
    toast(error.message, true);
  }
  state.readProgress = null;
  await loadSitePoints();
}

/** Reads both devices afresh, one after the other, so that the values compared are of now. */
export async function readBoth() {
  const left = state.device.instance;
  const { other } = state.compare;
  state.compare.reading = true;
  render();
  try {
    for (const instance of [left, other]) await readDevice(instance);
    const last = await api.lastPoints(left);
    if (last && state.device?.instance === left) state.points = last.points;
  } catch (error) {
    toast(error.message, true);
  }
  state.readProgress = null;
  if (state.compare) state.compare.reading = false;
  await loadSitePoints();
}

/** A point of the open device opens beside the comparison; one of the other device opens there. */
export function openComparedPoint(device, key) {
  if (device === state.device.instance) return selectPoint(key);
  state.compare = null;
  return openPointAt(device, key);
}
