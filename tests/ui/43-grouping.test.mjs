import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A site's own grouping pattern (docs/concept-toolbox.md part 4): typed with a preview, kept with
// the site over a reload, shown in the list and the tree, and given up for the automatic grouping.
export default {
  name: "Own grouping pattern per site",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click("[data-act=more-menu]");
    await page.click("[data-act=grouping-edit]");
    await page.waitForSelector("#grouping-pattern");
    await page.fill("#grouping-pattern", "^(Room");
    await page.waitForSelector(".grouping-panel .strip.err");
    assert.equal(await page.isDisabled("[data-act=grouping-save]"), true, "a broken pattern is not kept");
    await page.fill("#grouping-pattern", "^(Room \\w+) ");
    await page.waitForFunction(() => /10 of \d+ points fit/.test(document.querySelector(".grouping-panel").textContent));
    assert.match(await page.textContent(".grouping-preview"), /Room temp 2\.01 → Room temp · 2\.01/);
    await page.click("[data-act=grouping-save]");
    await page.waitForSelector(".grouping-panel", { state: "detached" });
    await page.waitForSelector(".row.group[data-group='part:Room temp']");
    // kept with the site
    await page.reload();
    await page.waitForSelector(".row.group[data-group='part:Room temp']");
    await page.click("[data-act=more-menu]");
    await page.click("[data-act=grouping-edit]");
    assert.equal(await page.inputValue("#grouping-pattern"), "^(Room \\w+) ");
    await page.click("[data-act=grouping-auto]");
    await page.waitForSelector(".grouping-panel", { state: "detached" });
    await page.waitForSelector(".row.group[data-group='kind:inputs']");
    assert.equal(await page.locator(".row.group[data-group='part:Room temp']").count(), 0);
  },
};
