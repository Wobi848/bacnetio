import { assert, open } from "./helpers.mjs";

// MS/TP in the search settings (docs/concept-mstp.md, step 2): the connection type, the simulated
// device's terminal as a port, and listening, which names the speed, the stations and a free
// address. The simulator runs on Linux only; elsewhere this test has nothing to listen to.
export default {
  name: "MS/TP: choose the port, listen to the trunk",
  async run({ page, base, api }) {
    const ports = await api("serial-ports");
    if (!ports.some((port) => port.description === "Simulator")) return;
    await open(page, base);
    await page.click("#search-settings");
    // BACnet/IP is chosen at first, with its fields
    assert.equal(await page.getAttribute("#link-ip", "aria-pressed"), "true");
    await page.waitForSelector("#adapter");
    await page.click("#link-mstp");
    assert.equal(await page.getAttribute("#link-mstp", "aria-pressed"), "true");
    assert.equal(await page.locator("#adapter").count(), 0, "no IP fields with MS/TP");
    // USB adapters first: the simulator stands for one
    await page.waitForSelector("#mstp-port");
    const chosen = await page.$eval("#mstp-port", (select) => select.selectedOptions[0].textContent);
    assert.match(chosen, /Simulator/);
    assert.equal(await page.$eval("#mstp-baud", (select) => select.value), "", "the speed is found by listening");
    // listening is the next step: the one filled button; searching is not offered yet
    assert.equal(await page.locator(".search-settings [data-act=find]").count(), 0);
    await page.click(".search-settings [data-act=mstp-listen]");
    await page.waitForSelector(".mstp-heard dl", { timeout: 15000 });
    const heard = await page.textContent(".mstp-heard");
    // a pseudo-terminal has no speed of its own: the first speed tried hears the device
    assert.match(heard, /38400/);
    assert.match(heard, /Stations heard\s*5\b/);
    assert.match(heard, /\d+ is free/);
    // back to BACnet/IP: its fields and its next step again
    await page.click("#link-ip");
    await page.waitForSelector("#adapter");
    assert.equal(await page.locator(".search-settings [data-act=find]").count(), 1);
  },
};
