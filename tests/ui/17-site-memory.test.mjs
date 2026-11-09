import { assert, open, openDevice, searchInUi, showSite } from "./helpers.mjs";

// After bacnetio is closed and started again, the devices and the points read show at once,
// marked as remembered, with "Search again" as the next step.
export default {
  name: "Site memory after a restart",
  async run({ page, base, restart }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await restart();
    await page.reload();
    await page.waitForSelector(".row[data-point]", { timeout: 15000 });
    assert.equal(await page.textContent(".pagehead h1"), "AHU-01");
    assert.equal(await page.$$eval(".row[data-point]", (rows) => rows.length), 24);
    // what is on the screen can be handed over, without reading anything again
    const name = await page.textContent(".row[data-point] .name");
    const instance = await page.evaluate(() => window.location.hash.match(/\d+/)?.[0] ?? "1001");
    for (const path of [`/api/devices/${instance}/points.csv`, `/api/devices/${instance}/points.ede.csv`]) {
      const answer = await page.request.get(`${base}${path}`);
      assert.equal(answer.status(), 200, path);
      assert.ok((await answer.text()).includes(name.trim()), `${path} holds the points`);
    }
    assert.match(await (await page.request.get(`${base}/api/devices.csv`)).text(), /AHU-01/);
    assert.ok(await page.$eval("#toast", (toast) => toast.hidden), "no error on opening a remembered device");
    await showSite(page);
    await page.waitForSelector(".strip.info");
    assert.match(await page.textContent(".strips"), /Remembered from/);
    assert.equal(await page.textContent("#next"), "Search again");
    await page.click("#next");
    // the note of the memory goes; a note bacnetio worked around (blue as well) may come with the search
    await page.waitForFunction(() => !/Remembered from/.test(document.querySelector(".strips")?.textContent ?? "") && !document.querySelector(".progress"), null, { timeout: 30000 });
    assert.equal(await page.$$eval(".row[data-device]", (rows) => rows.length), 2);
  },
};
