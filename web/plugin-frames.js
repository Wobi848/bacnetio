// The frames plugins run in. They live beside the work area, not in it: the work area is drawn anew
// all the time, and a frame drawn anew would start its plugin again.

import { PLUGIN_AREA } from "./plugins.js";

const frames = new Map();
/** The devices the plugins last heard of, to tell them when a search or a site changes them. */
let toldDevices = "";

/** The plugin a message came from, or null for anything else. */
export function pluginOfWindow(source) {
  for (const [id, frame] of frames) if (frame.contentWindow === source) return id;
  return null;
}

function frameFor(host, id) {
  const frame = document.createElement("iframe");
  // also sandboxed by the page's own policy (server); the frame says it again for older browsers
  frame.setAttribute("sandbox", "allow-scripts");
  frame.className = "plugin-frame";
  frame.title = id;
  frame.hidden = true;
  frame.src = `/api/plugins/${encodeURIComponent(id)}/page`;
  host.append(frame);
  return frame;
}

/**
 * One frame per plugin switched on, started with bacnetio, so its marks are there before its page
 * is opened; the one of the open area shows, and the work area hides.
 */
export function syncFrames(state) {
  const host = document.getElementById("plugin-frames");
  const on = new Set(state.plugins.filter((plugin) => plugin.on).map((plugin) => plugin.id));
  for (const [id, frame] of frames) {
    if (on.has(id)) continue;
    frame.remove();
    frames.delete(id);
  }
  const open = state.area.startsWith(PLUGIN_AREA) && !state.siteSearch.trim() ? state.area.slice(PLUGIN_AREA.length) : null;
  for (const id of on) if (!frames.has(id)) frames.set(id, frameFor(host, id));
  for (const [id, frame] of frames) frame.hidden = id !== open || !on.has(open);
  tellDevices(state);
  const showing = Boolean(open && on.has(open));
  host.hidden = !showing;
  document.getElementById("work").hidden = showing;
}

/** Plugin API 1 event: `{ event: "devices" }` when the devices to ask for change; a plugin asks again. */
function tellDevices(state) {
  const now = (state.scan.devices ?? []).map(({ instance, name, address }) => `${instance} ${name} ${address}`).join("\n");
  if (now === toldDevices) return;
  toldDevices = now;
  for (const frame of frames.values()) frame.contentWindow?.postMessage({ event: "devices" }, "*");
}
