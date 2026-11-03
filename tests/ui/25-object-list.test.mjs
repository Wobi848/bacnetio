import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The object list: columns of their own, search marked, all groups at once, values live.
export default {
  name: "Object list: columns, marked search, groups, live values",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    const row = ".row[data-point]:has-text('Supply air temp')";
    assert.match(await page.textContent(`${row} .desc`), /Supply duct/);
    assert.equal((await page.textContent(`${row} .id`)).trim(), "1");
    await page.fill("#filter", "duct");
    assert.equal(await page.textContent(`${row} .desc mark`), "duct");
    await page.fill("#filter", "");
    await page.click("button[data-act=more-menu]");
    await page.click("button[data-act=groups-all][data-open=false]");
    assert.equal(await page.$(".row[data-point]"), null, "all groups closed");
    await page.click("button[data-act=more-menu]");
    await page.click("button[data-act=groups-all][data-open=true]");
    await page.waitForSelector(row);
    // live values: a change on the device reaches the list by itself
    await page.click("button[data-act=live-list]");
    assert.match(await page.textContent("button[data-act=live-list]"), /on/);
    await api("devices/1001/objects/1/1/write", "POST", { value: "63", priority: 9 });
    await page.waitForFunction(() => /63/.test(document.querySelector(".row[data-point][title='Heating valve'] .value")?.textContent ?? ""), null, { timeout: 15000 });
    await page.click("button[data-act=live-list]");
    await api("devices/1001/objects/1/1/release", "POST", { priority: 9 });
    const tooMany = await page.request.post(`${base}/api/devices/1001/values`, { data: { objects: Array.from({ length: 201 }, (_, instance) => ({ type: 0, instance })) } });
    assert.equal(tooMany.status(), 400);
  },
};
