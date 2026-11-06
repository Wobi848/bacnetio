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
    // a note on what bacnetio already worked around informs, it does not warn; one to act on stays orange
    const strips = await page.$$eval(".strips .strip", (all) => all.map((strip) => [strip.className, strip.textContent]));
    for (const [kind, text] of strips) {
      if (/works around it|asks every address|answered only when asked directly/.test(text)) assert.match(kind, /\binfo\b/, text);
      if (/port is in use/.test(text)) assert.doesNotMatch(kind, /\binfo\b/, text);
    }
    // the name is the data: shown whole in a narrow list, where the vendor gives way
    await page.setViewportSize({ width: 1366, height: 768 });
    const cut = await page.$$eval(".row[data-device] .name", (cells) => cells.filter((cell) => cell.scrollWidth > cell.clientWidth).length);
    assert.equal(cut, 0);
    // the search also knows each device's objects, without anyone opening a device
    const objects = await page.$$eval(".row[data-device] .objects", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(objects.every((text) => /: [1-9]\d*$/.test(text)), `object counts: ${objects}`);
  },
};
