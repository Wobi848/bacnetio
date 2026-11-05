// The network adapters, read again while the page is open: a new address, a VPN that came up or a
// cable plugged in shows without restarting bacnetio.

import { api } from "./api.js";
import { chooseAdapter } from "./format.js";
import { render, state } from "./store.js";

const ADAPTER_POLL_MS = 5000;

/** Reads the adapters; keeps `wanted` where it is still there. Draws anew only on a change. */
export async function loadAdapters(wanted = state.adapter) {
  const adapters = await api.adapters();
  if (JSON.stringify(adapters) === JSON.stringify(state.adapters)) return;
  state.adapters = adapters;
  state.adapter = chooseAdapter(adapters, wanted);
  render();
}

async function refresh() {
  // an open adapter list would close under the pointer; the next round catches up
  if (document.visibilityState !== "visible" || document.activeElement?.id === "adapter") return;
  try {
    await loadAdapters();
  } catch {
    // bacnetio was closed; the next round tries again
  }
}

/** Every few seconds while the page is in view, and at once when it comes back (another window, the phone's home screen). */
export function watchAdapters() {
  setInterval(refresh, ADAPTER_POLL_MS);
  window.addEventListener("focus", refresh);
  document.addEventListener("visibilitychange", refresh);
}
