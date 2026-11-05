// Whether a new version is out (docs/concept-update-check.md): asked at most once a day and only
// while the user lets it; the answer is kept in this browser until the next check.

import { api } from "./api.js";
import { UPDATE_CHOICE_KEY, updateCheckOn } from "./settings.js";
import { render, state } from "./store.js";

const RESULT_KEY = "bacnetio.update";
const DAY_MS = 24 * 60 * 60 * 1000;

function keep(key, value) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // without storage the page asks again at the next start
  }
}

function lastResult() {
  try {
    return JSON.parse(localStorage.getItem(RESULT_KEY));
  } catch {
    return null;
  }
}

/** Asks the server unless a check of the last day by this version is kept, or `now` is set by the user. */
export async function checkForUpdate(now = false) {
  if (!updateCheckOn()) return;
  const kept = lastResult();
  // after an update the kept answer is the old version's: it would offer the version now running
  if (!now && kept && kept.current === state.version && Date.now() - kept.at < DAY_MS) {
    state.update = kept;
    render();
    return;
  }
  state.update = { ...state.update, checking: true };
  render();
  try {
    const answer = await api.update();
    state.update = { at: Date.now(), current: state.version, newer: answer.newer, version: answer.latest?.version ?? null, url: answer.latest?.url ?? null };
  } catch (error) {
    state.update = { at: Date.now(), current: state.version, error: error.message };
  }
  keep(RESULT_KEY, JSON.stringify(state.update));
  render();
}

export function setUpdateCheck(on) {
  keep(UPDATE_CHOICE_KEY, on ? null : "off");
  if (on) {
    checkForUpdate(true);
    return;
  }
  keep(RESULT_KEY, null);
  state.update = null;
  render();
}
