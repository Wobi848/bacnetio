import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A trend log of the device shows as chart with its record count and a CSV export.
export default {
  name: "Trend log: chart and export",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Supply air temp log')");
    await page.waitForSelector(".detail .chart svg", { timeout: 15000 });
    assert.match(await page.textContent(".detail"), /96 records/);
    assert.ok(await page.$(".detail a[href$='trend.csv']"));
  },
};
