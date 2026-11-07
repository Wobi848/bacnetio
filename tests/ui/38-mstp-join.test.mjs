import { assert, open, openDevice } from "./helpers.mjs";

// MS/TP, step 3 (docs/concept-mstp.md): after listening, bacnetio joins the trunk at the address
// it proposes and searches it; an address heard while listening is refused; joining and leaving
// are in the log. The simulated device is station 5; Linux only, like 37-mstp.
export default {
  name: "MS/TP: join the trunk, find its device, leave",
  async run({ page, base, api }) {
    const ports = await api("serial-ports");
    if (!ports.some((port) => port.description === "Simulator")) return;
    await open(page, base);
    await page.click("#search-settings");
    await page.click("#link-mstp");
    await page.waitForSelector("#mstp-port");
    // joining is offered only once a speed was heard
    assert.equal(await page.locator("[data-act=mstp-search]").count(), 0);
    await page.click("[data-act=mstp-listen]");
    await page.waitForSelector("#mstp-station", { timeout: 15000 });
    // right above the station heard: taken into the ring at once
    assert.equal(await page.inputValue("#mstp-station"), "6");
    // an address in use is refused by the server, in words
    await page.fill("#mstp-station", "5");
    await page.click("[data-act=mstp-search]");
    await page.waitForFunction(() => /another device uses it/.test(document.body.textContent));
    // the proposed one is taken
    await page.click("#search-settings");
    await page.fill("#mstp-station", "6");
    await page.click("[data-act=mstp-search]");
    await page.waitForSelector(".row[data-device='2005']", { timeout: 30000 });
    assert.match(await page.textContent(".row[data-device='2005']"), /MSTP-05/);
    assert.match(await page.textContent(".side-trunk"), /station 6, 38400 baud/);
    // no device by IP on a trunk
    assert.equal(await page.locator("[data-act=locate-menu]").count(), 0);
    // the device's points are read over the trunk as over IP
    await openDevice(page, "MSTP-05");
    assert.match(await page.textContent(".row[data-point]:has-text('Room temperature')"), /21[.,]5/);
    // leaving closes the port; joining and leaving stand in the log
    await page.click("[data-act=mstp-leave]");
    await page.waitForFunction(() => !document.querySelector(".side-trunk"));
    await page.goto(`${base}/#/log`);
    await page.waitForFunction(() => /Left on/.test(document.body.textContent));
    const log = await page.textContent("main");
    assert.match(log, /MS\/TP trunk/);
    assert.match(log, /Joined on \S+ at 38400 baud as station 6/);
  },
};
