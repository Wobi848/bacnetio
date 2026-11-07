import { assert, open, openDevice } from "./helpers.mjs";

// A value held over an MS/TP trunk, after bacnetio was restarted (docs/concept-mstp.md): the held
// value says it sits on the trunk; releasing it asks to join the trunk first, as bacnetio never
// joins one by itself; on the trunk again, it is released. Linux only, like 37-mstp.

/** Listens to the simulator's trunk and joins it at the address proposed. */
async function joinTrunk(page) {
  await page.click("#search-settings");
  await page.click("#link-mstp");
  await page.waitForSelector("#mstp-port");
  await page.click("[data-act=mstp-listen]");
  await page.waitForSelector("#mstp-station", { timeout: 15000 });
  await page.click("[data-act=mstp-search]");
  await page.waitForSelector(".side-trunk", { timeout: 30000 });
}

export default {
  name: "MS/TP: a value held on the trunk after a restart",
  async run({ page, base, api, restart }) {
    const ports = await api("serial-ports");
    if (!ports.some((port) => port.description === "Simulator")) return;
    await open(page, base);
    await joinTrunk(page);
    await page.waitForSelector(".row[data-device='2005']", { timeout: 30000 });
    await openDevice(page, "MSTP-05");
    await page.click(".row[data-point]:has-text('Damper')");
    await page.waitForSelector(".detail .big");
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "30");
    await page.click("button[data-act=write]");
    await page.waitForSelector(".slots .slot.mine", { timeout: 15000 });
    // closed with the value still held: the trunk is left with the program
    await restart();
    // the page still shows the trunk; searching again finds bacnetio on none and asks to listen
    assert.equal(await page.locator(".side-trunk").count(), 1);
    await page.click("[data-act=find] >> visible=true");
    await page.waitForSelector(".search-settings #mstp-port");
    assert.equal(await page.getAttribute("#link-mstp", "aria-pressed"), "true");
    assert.equal(await page.locator(".side-trunk").count(), 0);
    await page.click(".search-settings [data-act=close-menu]");
    await page.goto(`${base}/#/log`);
    await page.waitForSelector(".row.held");
    assert.match(await page.textContent(".row.held"), /MS\/TP 5/);
    await page.click(".row.held [data-act=release-held]");
    await page.waitForFunction(() => /sits on an MS\/TP trunk as station 5: listen to the trunk and join it first/.test(document.body.textContent), null, { timeout: 15000 });
    assert.equal(await page.locator(".row.held").count(), 1, "still held: nothing was released");
    // on the trunk again, the release reaches the device
    await joinTrunk(page);
    await page.goto(`${base}/#/log`);
    await page.waitForSelector(".row.held");
    await page.click(".row.held [data-act=release-held]");
    await page.waitForFunction(() => !document.querySelector(".row.held"), null, { timeout: 30000 });
  },
};
