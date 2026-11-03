import { assert, open, searchInUi } from "./helpers.mjs";

// The overview's next step searches; devices fill in with names, the sidebar counts them.
export default {
  name: "Search: devices with names, count, notes",
  async run({ page, base }) {
    await open(page, base);
    assert.equal(await page.textContent("#next"), "Find devices");
    await searchInUi(page);
    const names = await page.$$eval(".row[data-device] .name", (cells) => cells.map((cell) => cell.textContent));
    assert.deepEqual(names.sort(), ["AHU-01", "DDC4000_#21"]);
    assert.equal(await page.textContent(".tabs a[data-area=workspace] .n"), "2");
    assert.match(await page.textContent(".pagehead p"), /2 devices in 127\.0\.0\.0\/29/);
    // the search also knows each device's objects, without anyone opening a device
    const objects = await page.$$eval(".row[data-device] .objects", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(objects.every((text) => /: [1-9]\d*$/.test(text)), `object counts: ${objects}`);
  },
};
