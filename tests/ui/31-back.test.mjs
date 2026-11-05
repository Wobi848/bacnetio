import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The Android back key calls bacnetioBack: one step back each time, and only with no step left
// does the app ask whether to leave.
const back = (page) => page.evaluate(() => JSON.parse(window.bacnetioBack()));

export default {
  name: "Back key steps back before the app asks to leave",
  async run({ page, base }) {
    await open(page, base, { area: "workspace" });
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Outdoor air temp')");
    await page.goto(`${base}/#/problems`);
    await page.waitForSelector("[data-keep=problems]");

    assert.deepEqual(await back(page), { stepped: true, held: 0 });
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
    await page.click(".row[data-point]:has-text('Outdoor air temp')");
    await page.waitForSelector(".detail.open");
    assert.deepEqual(await back(page), { stepped: true, held: 0 });
    await page.waitForSelector(".detail.open", { state: "detached" });
    assert.deepEqual(await back(page), { stepped: true, held: 0 });
    await page.waitForSelector(".row[data-device]");
    assert.deepEqual(await back(page), { stepped: false, held: 0 });
  },
};
