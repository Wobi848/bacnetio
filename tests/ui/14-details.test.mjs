import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A point's details: live value by COV, all properties, reading the device again, adapter choice.
export default {
  name: "Point details: live, all properties, read again, adapter",
  async run({ page, base }) {
    await open(page, base);
    // the adapters to choose from are in the search settings
    await page.click(".side button[data-act=network]");
    const adapters = await page.$$eval("#adapter option", (options) => options.map((option) => option.textContent));
    assert.ok(adapters.some((label) => label.includes("127.0.0.1")), adapters.join(", "));
    await page.click("button[data-act=close-menu]");
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Supply air temp')");
    await page.waitForSelector(".detail .chip:has-text('Live')", { timeout: 15000 });
    // asking each time instead of following COV: the live mark goes
    await page.click("button[data-act=live-mode][data-mode=read]");
    await page.waitForFunction(() => !document.querySelector(".detail .chip.ok"));
    await page.waitForTimeout(2500);
    assert.equal(await page.$(".detail .chip:has-text('Live')"), null);
    await page.click("button[data-act=live-mode][data-mode=auto]");
    await page.waitForSelector(".detail .chip:has-text('Live')", { timeout: 15000 });
    await page.click(".detail button[data-act=technical]");
    await page.waitForFunction(() => document.querySelectorAll(".detail .props dt").length > 8, null, { timeout: 15000 });
    // in sections and plain words: names, states and yes or no by their meaning
    const plain = await page.textContent(".detail");
    assert.match(plain, /Value[\s\S]*Present value[\s\S]*State\s*Normal/);
    assert.match(plain, /Out of service\s*No/);
    assert.doesNotMatch(plain, /present-value|structured value/);
    // the properties outside the sections wait behind their own button
    await page.fill("#property-filter", "units");
    assert.equal(await page.$$eval(".detail .props dt", (labels) => labels.map((label) => label.textContent.trim())).then((labels) => labels.join("|")), "Unit");
    await page.fill("#property-filter", "");
    // an input has no value to write, so the properties offer the unlock themselves
    assert.equal(await page.$("button[data-act=property-edit]"), null);
    await page.click("button[data-act=ask-properties]");
    await page.click("text=Allow changes for this session");
    await page.waitForSelector("button[data-act=property-edit]");
    await page.click("button[data-act=reload]");
    await page.waitForFunction(() => !document.querySelector("button[data-act=reload]")?.disabled);
    // 19 data points and the three Structured View objects of the simulator's plant view
    assert.equal(await page.$$eval(".row[data-point]", (rows) => rows.length), 24);
    assert.ok(await page.$(".row[data-point].active"), "the chosen point stays chosen");
  },
};
