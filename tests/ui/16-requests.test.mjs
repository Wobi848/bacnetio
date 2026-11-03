import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The device's request log shows every request of the last reading with its duration: first the
// device's limits, then the object list and the values, none failed against the simulator.
export default {
  name: "Request log of a reading",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "DDC4000_#21");
    await page.click("button[data-act=requests]");
    await page.waitForSelector(".detail .requests dd");
    const log = await page.textContent(".detail .requests");
    assert.match(log, /max-apdu-length-accepted/);
    assert.match(log, /ReadPropertyMultiple, \d+ objects/);
    assert.match(log, /\d+ ms/);
    assert.equal(await page.$(".detail .requests .chip.err"), null, "a request failed");
    // opening a device takes the object list from the search; reading again asks the device
    assert.match(log, /Object list from the search, \d+ entries/);
    await api("devices/21/read", "POST");
    await page.waitForFunction(async () => {
      const job = await (await fetch("/api/devices/21/read")).json();
      return job.state === "done";
    }, null, { timeout: 30000 });
    const again = await api("devices/21/read");
    const asked = again.requests.map((request) => request.what).join(" | ");
    assert.match(asked, /object-list/);
    assert.doesNotMatch(asked, /from the search/);
  },
};
