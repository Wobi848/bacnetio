import { assert, open, searchInUi } from "./helpers.mjs";

// Number keys switch areas, / jumps to the search, arrows move the chosen row, Enter opens.
export default {
  name: "Keyboard: areas, search, arrows, Enter",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await page.keyboard.press("4");
    assert.equal(await page.evaluate(() => location.hash), "#/recording");
    await page.keyboard.press("1");
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
    await page.keyboard.press("/");
    assert.equal(await page.evaluate(() => document.activeElement.id), "site-search");
    await page.keyboard.press("Escape");
    await page.click(".pagehead");
    await page.keyboard.press("ArrowDown");
    await page.waitForSelector(".row[data-device].active");
    await page.keyboard.press("Enter");
    await page.waitForSelector(".row[data-point], .row.group", { timeout: 30000 });
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
  },
};
