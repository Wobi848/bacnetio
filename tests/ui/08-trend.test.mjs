import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A trend log of the device shows as chart with its record count and a CSV export; a log of several
// points shows a chart per point, named as the point list read it.
export default {
  name: "Trend log and trend log multiple: chart and export",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Supply air temp log')");
    await page.waitForSelector(".detail .chart svg", { timeout: 15000 });
    assert.match(await page.textContent(".detail"), /96 records/);
    assert.ok(await page.$(".detail a[href$='trend.csv']"));
    await page.click(".row[data-point]:has-text('Air handling log')");
    await page.waitForFunction(() => document.querySelectorAll(".detail .chart svg").length === 3, null, { timeout: 15000 });
    const detail = await page.textContent(".detail");
    assert.match(detail, /96 records of 3 points/);
    for (const name of ["Supply air temp", "Heating valve", "Supply fan enable"]) assert.match(detail, new RegExp(name));
    const csv = await (await page.request.get(`${base}/api/devices/1001/objects/27/1/trend.csv`)).text();
    assert.match(csv.split("\n")[0], /Time \(device\).*Supply air temp.*Heating valve.*Supply fan enable/);
    assert.equal(csv.trim().split("\n").length, 97);
  },
};
