import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Two devices compared (docs/concept-toolbox.md part 2): the second one is chosen and read when it
// is not yet, the views count the rows, a point of the open device opens beside, and the
// comparison closes back to the list.
const count = async (page, view) => Number(await page.textContent(`[data-act=compare-view][data-view=${view}] .n`));

export default {
  name: "Compare two devices",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click("[data-act=more-menu]");
    await page.click("[data-act=compare-start]");
    await page.waitForSelector(".compare-choices");
    await page.click(".compare-choices [data-act=compare-choose]:has-text('DDC4000')");
    await page.waitForSelector(".compare-table", { timeout: 30000 });
    const all = await count(page, "all");
    assert.equal(await page.locator(".compare-table tbody tr").count(), all, "every row in the view All");
    assert.ok(await count(page, "onlyLeft") > 0 && await count(page, "onlyRight") > 0, "other names stand alone");
    await page.click("[data-act=compare-view][data-view=onlyRight]");
    assert.equal(await page.locator(".compare-table tbody tr").count(), await count(page, "onlyRight"));
    await page.click("[data-act=compare-view][data-view=onlyLeft]");
    await page.click(".compare-table [data-act=compare-open][data-device='1001'] >> nth=0");
    await page.waitForSelector(".detail .big");
    assert.equal(await page.locator(".compare-table").count(), 1, "the comparison stays beside the details");
    await page.click("[data-act=compare-read]");
    await page.waitForFunction(() => !document.querySelector("[data-act=compare-read]")?.disabled);
    await page.click("[data-act=compare-close]");
    await page.waitForSelector("#filter");
    assert.equal(await page.locator(".compare-table").count(), 0);
  },
};
