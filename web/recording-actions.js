// The recording's actions: adding points, starting and stopping, keeping the page fresh.

import { api } from "./api.js";
import { t } from "./i18n.js";
import { recordingKey } from "./recording.js";
import { render, state, toast } from "./store.js";

const RECORDING_POLL_MS = 2000;

/** Runs a recording request; every one answers with the whole recording. */
export async function recordingRequest(request, done) {
  try {
    state.recording = await request();
    if (done) toast(done);
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

export const recordHere = (add) => recordPoint(state.device.instance, state.object, add);

export function recordPoint(device, { type, instance }, add) {
  recordingRequest(
    () => (add ? api.recordAdd(device, type, instance) : api.recordRemove(device, type, instance)),
    add ? t("rec.added") : null,
  );
}

/** Poll at the interval, or record every report of the device (COV). */
export function setRecordMode(key, mode) {
  const point = state.recording.points.find((candidate) => recordingKey(candidate) === key);
  recordingRequest(() => api.recordMode(point.device, point.type, point.instance, mode));
}

export function removeRecorded(key) {
  const point = state.recording.points.find((candidate) => recordingKey(candidate) === key);
  recordingRequest(() => api.recordRemove(point.device, point.type, point.instance));
}

/** Keeps the recording fresh while its page is open or it runs (status bar). */
export async function pollRecording() {
  if (state.area === "recording" || state.recording?.running) {
    try {
      const recording = await api.recording();
      // drawn anew only when something moved: a click that falls into a redraw is lost
      const changed = recording.running || JSON.stringify(recording) !== JSON.stringify(state.recording);
      state.recording = recording;
      if (changed && state.area === "recording" && !state.onChart) render();
    } catch {
      // bacnetio was closed; the next round tries again
    }
  }
  setTimeout(pollRecording, RECORDING_POLL_MS);
}
