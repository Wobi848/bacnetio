// Starting, stopping and clearing the capture, and keeping its count fresh while it runs.

import { api } from "./api.js";
import { render, state, toast } from "./store.js";

/** How often the count of a running capture is asked for. */
const CAPTURE_POLL_MS = 1000;
let polling = false;

export async function loadCapture() {
  try {
    state.capture = await api.capture();
  } catch (error) {
    toast(error.message, true);
  }
  render();
  if (state.capture?.running) pollCapture();
}

async function pollCapture() {
  if (polling) return;
  polling = true;
  while (state.capture?.running) {
    await new Promise((done) => setTimeout(done, CAPTURE_POLL_MS));
    try {
      const capture = await api.capture();
      const changed = JSON.stringify(capture) !== JSON.stringify(state.capture);
      state.capture = capture;
      // drawn anew only on its own page: a click that falls into a redraw is lost; the status bar
      // takes the count with the next redraw
      if (changed && state.area === "settings") render();
    } catch {
      // bacnetio was closed; the next round tries again
    }
  }
  polling = false;
}

/** One of start, stop and clear, with the capture's state as the answer. */
export async function captureRequest(action) {
  try {
    state.capture = await api.captureAction(action);
  } catch (error) {
    toast(error.message, true);
  }
  render();
  if (state.capture?.running) pollCapture();
}
