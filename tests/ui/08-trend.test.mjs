import { readFile } from "node:fs/promises";
import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A trend log of the device shows as chart with its record count and a CSV export; a log of several
// points shows a chart per point, named as the point list read it; an event log lists past alarms;
// a file is saved whole.
export default {
  name: "The device's own logs and files: chart, list, export, save",
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
    // the device's own list of past alarms, newest first, with the points' names and texts
    await page.click(".row[data-point]:has-text('Alarm history')");
    await page.waitForSelector(".detail .alarm.logged", { timeout: 15000 });
    const logged = await page.$$eval(".detail .alarm.logged", (rows) => rows.map((row) => row.textContent.replace(/\s+/g, " ").trim()));
    assert.equal(logged.length, 4, logged.join(" | "));
    assert.match(logged[0], /Supply air temp.*Acknowledged/);
    assert.match(logged[1], /Supply air temp.*Supply air normal.*Normal/i);
    assert.match(logged[3], /Filter alarm.*Filter dirty/);
    assert.match(await page.textContent(".detail"), /4 entries, newest first/);
    const events = await (await page.request.get(`${base}/api/devices/1001/objects/25/1/events.csv`)).text();
    assert.match(events, /Supply air too warm/);
    assert.equal(events.trim().split("\n").length, 5);
    // a file the device keeps is saved whole under its name, read only
    await page.click(".row[data-point]:has-text('Parameters.csv')");
    await page.waitForSelector("button[data-act=save-file]");
    assert.match(await page.textContent(".detail"), /Nothing is written to the device/);
    const [download] = await Promise.all([page.waitForEvent("download"), page.click("button[data-act=save-file]")]);
    assert.equal(download.suggestedFilename(), "Parameters.csv");
    const saved = await readFile(await download.path(), "utf8");
    assert.equal(saved, "name;value;unit\nsupply air setpoint;20.0;degC\nheating valve limit;100;%\nfan start;06:00;\nfan stop;22:00;\n");
  },
};
