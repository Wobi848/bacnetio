import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A controller with 170+ points named as paths: grouped by module, groups closed, search opens
// the matching ones, multi-state values show their texts.
export default {
  name: "Points: groups by module, search, state texts, details",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "DDC4000_#21");
    const count = Number(await page.textContent(".seg [data-view=all] .n"));
    assert.ok(count >= 170, `${count} points`);
    assert.ok(await page.$$eval(".row[data-point]", (rows) => rows.length) >= 170, "a device opens showing its values");
    // keys: to the last row and back, and into the list's own search
    await page.keyboard.press("End");
    await page.waitForFunction(() => [...document.querySelectorAll(".row[data-point]")].at(-1).classList.contains("active"));
    await page.keyboard.press("Home");
    await page.waitForFunction(() => document.querySelector(".row[data-point]").classList.contains("active"));
    await page.keyboard.press("f");
    assert.equal(await page.evaluate(() => document.activeElement.id), "filter");
    await page.evaluate(() => document.activeElement.blur());
    const groups = await page.$$eval(".row.group .name", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(groups.includes("SY_Datsi.01"), groups.join(", "));
    await page.click(".row[data-point]:has-text('state')");
    await page.waitForSelector(".detail .big");
    assert.equal((await page.textContent(".detail .big")).trim(), "Running");
    await page.fill("#filter", "Zone.07 temp");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point]").length === 1);
    assert.match(await page.textContent(".row[data-point]"), /Out of service/);
  },
};
