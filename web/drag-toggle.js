// Switching watch or record on many points at once: press on one switch in the point list and
// drag over the other rows, as in Blender (docs/concept-drag-toggle.md). Mouse only; touch scrolls,
// and a key or a tap stays a plain click (app.js).

import { api } from "./api.js";
import { isRecorded, isWatched, pointKey } from "./format.js";
import { recordingRequest } from "./recording-actions.js";
import { state } from "./store.js";
import { unwatchPoint, watchPoint } from "./watch-actions.js";

let drag = null;
let swallowClick = false;
let recordings = Promise.resolve();

function setWatched(device, point, on) {
  if (on) watchPoint(device, point);
  else unwatchPoint(device, point);
}

function setRecorded(device, { type, instance }, on) {
  // one after the other: each answer is the whole recording, and a late one must not undo a newer one
  recordings = recordings.then(() =>
    recordingRequest(() => (on ? api.recordAdd(device, type, instance) : api.recordRemove(device, type, instance))));
}

const SWITCHES = {
  "quick-watch": { isOn: isWatched, set: setWatched },
  "quick-record": { isOn: isRecorded, set: setRecorded },
};

/** The row under the pointer takes the state of the first switch, once per drag. */
function pass(row) {
  const key = row?.dataset.point;
  if (!key || drag.passed.has(key)) return;
  drag.passed.add(key);
  // rows without the switch (types that cannot be recorded) are passed over
  if (!row.querySelector(`button[data-act="${drag.act}"]`)) return;
  const point = state.points?.find((candidate) => pointKey(candidate) === key);
  if (!point) return;
  const device = state.device.instance;
  const kind = SWITCHES[drag.act];
  if (kind.isOn(state, device, point) !== drag.on) kind.set(device, point, drag.on);
}

export function installDragToggle() {
  document.addEventListener("pointerdown", (event) => {
    if (event.pointerType !== "mouse" || event.button !== 0) return;
    const button = event.target.closest("button.quick[data-act]");
    if (!button || !SWITCHES[button.dataset.act]) return;
    // no text selection while dragging
    event.preventDefault();
    drag = { act: button.dataset.act, on: button.getAttribute("aria-pressed") !== "true", passed: new Set() };
    swallowClick = true;
    pass(button.closest(".row[data-point]"));
  });
  document.addEventListener("pointermove", (event) => {
    if (drag) pass(event.target.closest?.(".row[data-point]"));
  });
  document.addEventListener("pointerup", () => {
    if (!drag) return;
    drag = null;
    // the click that follows the release was already handled by the press
    setTimeout(() => { swallowClick = false; });
  });
  document.addEventListener("click", (event) => {
    if (!swallowClick) return;
    swallowClick = false;
    event.stopPropagation();
    event.preventDefault();
  }, true);
}
