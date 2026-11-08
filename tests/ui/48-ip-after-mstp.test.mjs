import { assert, open, openDevice } from "./helpers.mjs";

// On the MS/TP trunk, then a search over IP: it leaves the trunk as "Leave the trunk" does. With a
// value held it asks first; leaving is in the log, and the sidebar no longer shows the trunk.
// Linux only, like 37-mstp.
async function searchOverIp(page) {
  await page.click("#search-settings");
  await page.click("#link-ip");
  await page.click("#dialog [data-act=find]");
}

export default {
  name: "IP after MS/TP: the search leaves the trunk",
  async run({ page, base, api }) {
    const ports = await api("serial-ports");
    if (!ports.some((port) => port.description === "Simulator")) return;
    await open(page, base);
    await page.click("#search-settings");
    await page.click("#link-mstp");
    await page.click("[data-act=mstp-listen]");
    await page.waitForSelector("#mstp-station", { timeout: 15000 });
    await page.click("[data-act=mstp-search]");
    await page.waitForSelector(".row[data-device='2005']", { timeout: 30000 });
    // a value held on the trunk: the search over IP asks first and does not start
    await openDevice(page, "MSTP-05");
    await page.click(".row[data-point]:has-text('Damper')");
    await page.waitForSelector(".detail .big");
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "40");
    await page.click("button[data-act=write]");
    await page.waitForSelector(".slots .slot.mine", { timeout: 15000 });
    await searchOverIp(page);
    assert.match(await page.textContent(".side-trunk"), /You still hold a value/);
    assert.ok(await api("mstp/trunk"), "still on the trunk");
    // released, the search goes over IP and leaves the trunk
    await page.click(".side-trunk [data-act=release-all]");
    await page.waitForFunction(() => !/held/.test(document.querySelector(".status .chips")?.textContent ?? ""), null, { timeout: 15000 });
    await searchOverIp(page);
    await page.waitForSelector(".row[data-device='1001']", { timeout: 30000 });
    await page.waitForFunction(() => !document.querySelector(".side-trunk"));
    assert.equal(await api("mstp/trunk"), null);
    const log = await api("log");
    assert.ok(log.some((entry) => /^Left the MS\/TP trunk/.test(entry.value)), "leaving is in the log");
  },
};
