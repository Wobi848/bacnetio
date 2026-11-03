import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A reload keeps the area, the search result, the open device and the search settings.
export default {
  name: "Reload keeps area, devices, open device and settings",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.reload();
    await page.waitForSelector(".row[data-point]");
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
    assert.equal(await page.textContent(".pagehead h1"), "AHU-01");
    await page.click("button[data-act=network]");
    assert.equal(await page.inputValue("#subnet"), "127.0.0.0/29");
  },
};
