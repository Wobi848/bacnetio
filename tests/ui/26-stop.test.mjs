import { assert, open } from "./helpers.mjs";

// A big plant or a slow VPN: the brake must work, and what was found or read stays.
export default {
  name: "Stop a running search and reading",
  async run({ page, base }) {
    await open(page, base);
    const api = async (path) => (await page.request.get(`${base}/api/${path}`)).json();
    await page.click("#next");
    await page.click("button[data-act=stop-running]");
    await page.waitForFunction(() => !document.querySelector("button[data-act=stop-running]"), null, { timeout: 5000 });
    assert.equal((await api("scan")).state, "done");
    // without anything running, stopping changes nothing
    assert.equal((await page.request.post(`${base}/api/scan/stop`)).status(), 204);
    assert.equal((await page.request.post(`${base}/api/devices/1001/read/stop`)).status(), 204);
    assert.equal((await api("devices/1001/read")).state, "idle");
  },
};
