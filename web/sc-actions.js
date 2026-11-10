// BACnet/SC in the search settings (docs/concept-sc.md): choosing the certificate files, opening
// one with its password, connecting to the hub and leaving its network.

import { api } from "./api.js";
import { runSearch } from "./device-actions.js";
import { t } from "./i18n.js";
import { render, state, toast } from "./store.js";

/** The lines as the server checked them, and the hub bacnetio is on or was on last. */
export async function loadSc() {
  try {
    state.sc.view = await api.sc();
  } catch (error) {
    toast(error.message, true);
    return;
  }
  const { hub, failover_hub: failoverHub, last_hub: lastHub, last_failover_hub: lastFailoverHub } = state.sc.view;
  if (hub) {
    state.link = "sc";
    state.sc.hub = hub;
    state.sc.failoverHub = failoverHub ?? "";
    pollSc();
  } else if (!state.sc.hub && lastHub) {
    state.sc.hub = lastHub;
    state.sc.failoverHub = lastFailoverHub ?? "";
  }
  render();
}

const POLL_EVERY = 5000;
let polling = false;

/** While bacnetio is on an SC network, the state of its hub connection is asked for now and then. */
function pollSc() {
  if (polling) return;
  polling = true;
  (async () => {
    while (state.sc.view?.hub) {
      await new Promise((done) => setTimeout(done, POLL_EVERY));
      const before = JSON.stringify(state.sc.view?.link ?? null);
      try {
        state.sc.view = await api.sc();
      } catch (error) {
        toast(error.message, true);
        break;
      }
      if (JSON.stringify(state.sc.view.link ?? null) !== before) render();
    }
    polling = false;
  })();
}

/** Sends a file chosen for a line; the server reads it and says what it found. */
export async function chooseScFile(line, file) {
  if (!file) return;
  try {
    state.sc.view = await api.scFile(line, file);
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** Opens the file waiting on a line with the password typed; the field is emptied either way. */
export async function openScFile(line) {
  const password = state.sc.passwords[line] ?? "";
  state.sc.passwords[line] = "";
  try {
    state.sc.view = await api.scPassword(line, password);
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** Connects to the hub with the files chosen and searches the SC network. */
export async function searchOnSc() {
  const { hub, failoverHub, rememberKey } = state.sc;
  state.menu = null;
  const joined = await runSearch(() => api.scSearch(hub.trim(), failoverHub.trim(), rememberKey));
  await loadSc();
  return joined;
}

/** Deletes the key kept on this computer; the certificates stay. */
export async function forgetScKey() {
  try {
    state.sc.view = await api.scForgetKey();
    toast(t("sc.keyForgotten"));
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** Leaves the SC network. */
export async function leaveSc() {
  try {
    await api.scLeave();
    state.sc.leaving = false;
    state.log = await api.log();
  } catch (error) {
    toast(error.message, true);
  }
  await loadSc();
}
