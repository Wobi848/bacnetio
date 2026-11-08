import { assert, open, searchByApi } from "./helpers.mjs";

// A search over IP, then one on the MS/TP trunk: the trunk's list holds the trunk's device only.
// The devices found over IP are not on the trunk, and reading them there could only fail.
// Linux only, like 37-mstp.
export default {
  name: "MS/TP after IP: only the trunk's devices",
  async run({ page, base, api }) {
    const ports = await api("serial-ports");
    if (!ports.some((port) => port.description === "Simulator")) return;
    await searchByApi(api);
    await open(page, base);
    await page.waitForSelector(".row[data-device='1001']");
    await page.click("#search-settings");
    await page.click("#link-mstp");
    await page.click("[data-act=mstp-listen]");
    await page.waitForSelector("#mstp-station", { timeout: 15000 });
    await page.click("[data-act=mstp-search]");
    await page.waitForSelector(".row[data-device='2005']", { timeout: 30000 });
    let scan = await api("scan");
    while (scan.state === "running") {
      await new Promise((done) => setTimeout(done, 200));
      scan = await api("scan");
    }
    assert.deepEqual(scan.devices.map((device) => device.instance), [2005]);
    assert.equal(await page.locator(".row[data-device='1001']").count(), 0);
  },
};
