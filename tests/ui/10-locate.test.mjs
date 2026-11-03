import { assert, open } from "./helpers.mjs";

// A device the search does not reach opens by its address.
export default {
  name: "Device by IP address",
  async run({ page, base }) {
    await open(page, base, { area: "devices" });
    await page.click("button[data-act=locate-menu]");
    await page.fill("#locate-address", "127.0.0.2:47809");
    await page.click("button[data-act=locate]");
    await page.waitForSelector(".row[data-point]", { timeout: 15000 });
    assert.equal(await page.textContent(".pagehead h1"), "AHU-01");
  },
};
