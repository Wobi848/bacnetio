// Live values in the object list: the rows on screen are read again and again, as long as asked for.
// Only those: a list of thousands must not keep a device busy.

import { api } from "./api.js";
import { pointKey } from "./format.js";
import { render, state, toast } from "./store.js";

const LIVE_POLL_MS = 5000;
/** Rows read per round: more than a screen holds, few enough for a small controller. */
const MOST_ROWS = 64;
let running = false;

function rowsOnScreen() {
  const list = document.querySelector(".rows[data-list=points]")?.getBoundingClientRect();
  if (!list) return [];
  return [...document.querySelectorAll(".row[data-point]")]
    .filter((row) => {
      const box = row.getBoundingClientRect();
      return box.bottom > list.top && box.top < list.bottom;
    })
    .slice(0, MOST_ROWS)
    .map((row) => row.dataset.point.split(":").map(Number));
}

export async function toggleLiveList() {
  state.liveList = !state.liveList;
  render();
  if (!state.liveList || running) return;
  running = true;
  while (state.liveList && state.device) {
    const objects = rowsOnScreen().map(([type, instance]) => ({ type, instance }));
    const device = state.device.instance;
    if (objects.length && state.area === "workspace" && !state.reading) {
      try {
        const fresh = await api.liveValues(device, objects);
        if (state.device?.instance === device && state.points) {
          const byKey = new Map(state.points.map((point) => [pointKey(point), point]));
          for (const point of fresh) {
            const known = byKey.get(pointKey(point));
            if (known) Object.assign(known, { value: point.value, status: point.status, error: point.error });
          }
          if (!document.activeElement?.matches?.("select") && !state.onChart) render();
        }
      } catch (error) {
        state.liveList = false;
        toast(error.message, true);
        render();
      }
    }
    await new Promise((done) => setTimeout(done, LIVE_POLL_MS));
  }
  running = false;
}
