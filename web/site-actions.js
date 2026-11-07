// The site as a whole: reading every device, exporting them, and the sites kept on disk.

import { api } from "./api.js";
import { saveMemory } from "./memory.js";
import { searchScope } from "./shell.js";
import { go, render, state, toast } from "./store.js";
import { t } from "./i18n.js";

const READ_POLL_MS = 400;
const pause = (ms) => new Promise((done) => setTimeout(done, ms));

export async function loadSitePoints() {
  try {
    state.sitePoints = await api.sitePoints();
    state.events = await api.events();
  } catch (error) {
    toast(error.message, true);
  }
  render();
}


/** Shows a kept site, or one brought in as a file, as remembered. */
async function showSite(change) {
  try {
    await change();
    state.scan = await api.scan();
    state.device = null;
    state.points = null;
    state.selectedDevice = null;
    state.selectedPoint = null;
    state.node = null;
    saveMemory(state);
    await Promise.all([loadSitePoints(), loadSites()]);
    go("workspace");
  } catch (error) {
    toast(error.message, true);
  }
}

export const openSite = (name) => showSite(() => api.openSite(name));

/** What is in the file, or a sentence of our own: a broken file must not show the engine's
 * parser message, which is English in every language. */
export function siteFromFile(text) {
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(t("sites.notJson"));
  }
}

export async function importSite(file) {
  await showSite(async () => api.importSite(siteFromFile(await file.text())));
}

/** Forgets a kept site for good, which the sites page asks about first. */
export async function deleteSite(name) {
  try {
    await api.deleteSite(name);
    toast(t("sites.deleted", { name }));
  } catch (error) {
    toast(error.message, true);
  }
  state.deletingSite = null;
  await loadSites();
}

/** Gives a kept site a name of the technician's; an empty name goes back to the network. */
export async function renameSite(name, title) {
  try {
    await api.renameSite(name, title);
    state.renamingSite = null;
  } catch (error) {
    toast(error.message, true);
  }
  await loadSites();
}

/** Keeps a grouping pattern with the open site; none groups by itself again. */
export async function groupSite(name, pattern) {
  try {
    await api.groupSite(name, pattern);
    state.groupingEdit = null;
  } catch (error) {
    toast(error.message, true);
  }
  await loadSites();
}

export async function loadSites() {
  try {
    // the site shown first, then the latest
    state.sites = (await api.sites()).sort((one, other) => Number(other.open) - Number(one.open));
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** Reads every device not read yet, one after the other, and says how many could not be read. */
export async function readAllDevices() {
  state.reading = true;
  state.stopping = false;
  render();
  let failed = 0;
  for (const device of state.scan.devices ?? []) {
    if (state.stopping) break;
    if ((await api.readProgress(device.instance)).state === "done") continue;
    state.readingDevice = device.instance;
    await api.startRead(device.instance);
    if (!(await waitForReading(device.instance))) failed += 1;
  }
  state.reading = false;
  state.readProgress = null;
  await loadSitePoints();
  return failed;
}

/** Stops the search or reading that runs now, and with it the walk over all devices. */
export async function stopRunning() {
  state.stopping = true;
  try {
    if (state.scan.state === "running") await api.stopScan();
    if (state.reading && state.readingDevice !== null) await api.stopRead(state.readingDevice);
  } catch (error) {
    toast(error.message, true);
  }
}

/** Reads what is not read yet, then downloads the site's EDE file and its state texts. */
export async function exportSiteEde() {
  try {
    const failed = await readAllDevices();
    if (state.stopping) return;
    const link = document.createElement("a");
    link.href = `/api/site.ede.csv?project=${encodeURIComponent(searchScope(state))}`;
    link.download = "";
    link.click();
    // browsers keep a second download from starting at the same moment
    await pause(300);
    link.href = "/api/site.ede-states.csv";
    link.click();
    if (failed) toast(t("ede.siteFailed", { n: failed }), true);
  } catch (error) {
    toast(error.message, true);
  }
}

/** True when the reading of the device ended well. */
async function waitForReading(instance) {
  for (;;) {
    const view = await api.readProgress(instance);
    state.readProgress = view;
    if (view.state !== "running") return view.state === "done";
    render();
    await pause(READ_POLL_MS);
  }
}
