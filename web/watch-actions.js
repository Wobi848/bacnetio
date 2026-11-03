// Keeping the watch list: adding and removing points, and their values while the page is open.

import { api } from "./api.js";
import { saveWatch } from "./memory.js";
import { render, state } from "./store.js";

const WATCH_POLL_MS = 2000;
let polling = false;

const same = (entry, device, object) => entry.device === device && entry.type === object.type && entry.instance === object.instance;

export const watchHere = () => watchPoint(state.device.instance, state.object);
export const unwatchHere = () => unwatchPoint(state.device.instance, state.object);

export function watchPoint(device, point) {
  const { type, instance, name, units, value, status } = point;
  state.watch.push({ device, type, instance, name, units, value, status });
  saveWatch(state.watch);
  render();
}

export function unwatchPoint(device, point) {
  state.watch = state.watch.filter((entry) => !same(entry, device, point));
  saveWatch(state.watch);
  render();
}

export function unwatch(index) {
  state.watch.splice(index, 1);
  saveWatch(state.watch);
  render();
}

/** One round over the list, one point after the other, as long as the page is open. */
export async function pollWatch() {
  if (polling) return;
  polling = true;
  while (state.area === "watch" && !state.siteSearch) {
    for (const entry of [...state.watch]) {
      try {
        const live = await api.value(entry.device, entry.type, entry.instance);
        entry.value = entry.units && /^-?\d/.test(live.value) ? `${live.value} ${entry.units}` : live.value;
        if (live.status !== null) entry.status = live.status;
        entry.error = null;
      } catch (error) {
        entry.error = error.message;
      }
    }
    if (state.area === "watch" && !state.siteSearch) render();
    await new Promise((done) => setTimeout(done, WATCH_POLL_MS));
  }
  polling = false;
}
