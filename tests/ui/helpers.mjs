// Shared steps of the browser tests.
import assert from "node:assert/strict";
export { assert };

/** The simulated devices: example, loopback address. Both share one port, each on its address. */
export const SIMULATORS = [["simulated_plant", "127.0.0.2"], ["simulated_controller", "127.0.0.3"]];
export const SIM_PORT = 47809;
/** Search settings that reach the simulators: loopback adapter, their port, their addresses. */
export const SEARCH = { subnet: "127.0.0.0/29", bbmd: "", port: String(SIM_PORT), locate: "", idFrom: "", idTo: "" };

/** Opens bacnetio in `language` with the search settings for the simulators. */
export async function open(page, base, { language = "en", area = "overview", theme = "light" } = {}) {
  await page.addInitScript(([lang, search, look]) => {
    // bacnetio's page only: a plugin's sandboxed frame has no storage to set
    if (window !== window.top) return;
    // only the first time: a test may change them and reload
    if (!localStorage.getItem("bacnetio.language")) localStorage.setItem("bacnetio.language", lang);
    if (!localStorage.getItem("bacnetio.theme")) localStorage.setItem("bacnetio.theme", look);
    if (!localStorage.getItem("bacnetio.session")) {
      localStorage.setItem("bacnetio.session", JSON.stringify({ settings: search, adapterIp: "127.0.0.1", device: null }));
    }
  }, [language, SEARCH, theme]);
  await page.goto(`${base}/#/${area}`);
  // on a narrow screen the adapter sits in the folded head, present but hidden
  await page.waitForSelector("#adapter", { state: "attached" });
}

/** Searches through the API, as the UI would, and waits until it is done. */
export async function searchByApi(api) {
  await api("scan", "POST", { interface: "127.0.0.1", port: SIM_PORT, subnet: SEARCH.subnet, sweep: true });
  for (let attempt = 0; attempt < 200; attempt++) {
    const scan = await api("scan");
    if (scan.state !== "running") return scan;
    await new Promise((done) => setTimeout(done, 100));
  }
  throw new Error("the search did not finish");
}

/** Back to the device list, from wherever the workspace is. */
export async function showSite(page) {
  await page.keyboard.press("1");
  await page.click(".node[data-act=tree-site]");
  await page.waitForSelector(".row[data-device]");
}

/** Clicks the page's next step and waits until both simulated devices are listed. */
export async function searchInUi(page) {
  await page.click("#next");
  await page.waitForFunction(() => document.querySelectorAll(".row[data-device]").length === 2 && !document.querySelector(".progress"), null, { timeout: 30000 });
}

/** Opens the data points of a device by its name in the device list. */
export async function openDevice(page, name) {
  await page.click(`.node[data-act=tree-device]:has-text("${name}")`);
  await page.waitForSelector(".row[data-point], .row.group", { timeout: 30000 });
  // the points fill in while the device is read; wait until it is done
  await page.waitForFunction(() => !document.querySelector(".status .dot.busy"), null, { timeout: 30000 });
}
