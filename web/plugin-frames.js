// The frames plugins run in. They live beside the work area, not in it: the work area is drawn anew
// all the time, and a frame drawn anew would start its plugin again.

import { PLUGIN_AREA } from "./plugins.js";

const frames = new Map();

/** The plugin a message came from, or null for anything else. */
export function pluginOfWindow(source) {
  for (const [id, frame] of frames) if (frame.contentWindow === source) return id;
  return null;
}

/** One frame per plugin opened and still on; the one of the open area shows, the work area hides. */
export function syncFrames(state) {
  const host = document.getElementById("plugin-frames");
  const on = new Set(state.plugins.filter((plugin) => plugin.on).map((plugin) => plugin.id));
  for (const [id, frame] of frames) {
    if (on.has(id)) continue;
    frame.remove();
    frames.delete(id);
  }
  const open = state.area.startsWith(PLUGIN_AREA) && !state.siteSearch.trim() ? state.area.slice(PLUGIN_AREA.length) : null;
  if (open && on.has(open) && !frames.has(open)) {
    const frame = document.createElement("iframe");
    // also sandboxed by the page's own policy (server); the frame says it again for older browsers
    frame.setAttribute("sandbox", "allow-scripts");
    frame.className = "plugin-frame";
    frame.title = open;
    frame.src = `/api/plugins/${encodeURIComponent(open)}/page`;
    host.append(frame);
    frames.set(open, frame);
  }
  for (const [id, frame] of frames) frame.hidden = id !== open || !on.has(open);
  const showing = Boolean(open && on.has(open));
  host.hidden = !showing;
  document.getElementById("work").hidden = showing;
}
