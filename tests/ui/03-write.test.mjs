import fs from "node:fs";
import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Writing needs the unlock, shows who holds the value, lands in the log and is released there.
export default {
  name: "Write: unlock, priority array, log, release",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".detail .big");
    // every priority says what it is for, BACnet's names where the standard gives one
    assert.equal(await page.textContent(".slots .slot:nth-child(8) .who"), "Manual operator");
    assert.equal(await page.textContent(".slots .slot:nth-child(3) .who"), "");
    assert.equal(await page.$("#new-value"), null, "no input before the unlock");
    await page.click("text=Change value");
    // in the narrowest details beside the list the question breaks into lines, it does not stick out
    const size = page.viewportSize();
    await page.setViewportSize({ width: 1200, height: size.height });
    await page.evaluate(() => document.documentElement.style.setProperty("--detail-width", "280px"));
    assert.ok(await page.locator(".detail").evaluate((detail) => detail.scrollWidth <= detail.clientWidth), "the unlock question fits the details");
    await page.evaluate(() => document.documentElement.style.removeProperty("--detail-width"));
    await page.setViewportSize(size);
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "42.5");
    await page.click("button[data-act=write]");
    await page.waitForSelector(".slots .slot.mine");
    // the list shows the new value at once, not the one from before
    assert.match(await page.textContent(".row[data-point]:has-text('Heating valve') .value"), /42\.5/);
    assert.match(await page.textContent(".status .chips"), /1 held/);
    // releasing is the next step, but a new search stays at hand beside it
    assert.equal(await page.textContent("#next"), "Release the held value");
    assert.equal(await page.textContent(".nextbar button[data-act=find]"), "Search again");
    // the head says what to do, with the count; only the next step is filled, a chosen view is not
    assert.equal(await page.textContent(".pagehead p"), "22 data points, 1 of them held by you. Choose one to see its details.");
    const filled = (selector) => page.locator(selector).evaluate((element) => getComputedStyle(element).backgroundColor);
    assert.notEqual(await filled(".toolbar .seg button[aria-pressed=true]"), await filled("#next"));
    // on a phone the head keeps the next step; the search moves into the toolbar, nothing is lost
    const wide = page.viewportSize();
    await page.setViewportSize({ width: 390, height: 844 });
    assert.ok(await page.isHidden(".nextbar button[data-act=find]"));
    assert.ok(await page.isVisible(".toolbar .toolbar-head-tool[data-act=find]"));
    assert.ok(await page.isVisible("#next"));
    await page.setViewportSize(wide);
    assert.ok(await page.isHidden(".toolbar .toolbar-head-tool"));
    // the problems and exports know the written value too, not the one from the last reading
    const site = await (await page.request.get(`${base}/api/site/points`)).json();
    assert.match(site["1001"].points.find((point) => point.name === "Heating valve").value, /42\.5/);
    await page.keyboard.press("5");
    await page.waitForSelector(".row.held button[data-act=release-held]");
    await page.waitForFunction(() => /wrote 42\.5 at priority 8/.test(document.querySelector(".rows").textContent));
    // the page says how much is held and what to do; the next step names it
    assert.match(await page.textContent(".pagehead .lead"), /^1 value you still hold in the plant\. Release it when you are done\.$/);
    assert.equal(await page.textContent("#next"), "Release the held value");
    // the held value: the point's name first, its value with the unit, the priority as a warning
    const held = page.locator(".row.held");
    assert.match(await held.locator(".name").textContent(), /^Heating valveAHU-01 · Analog output 1 · held since \d\d:\d\d$/);
    assert.equal(await held.locator(".val").textContent(), "42.5%");
    assert.equal(await held.locator(".chip.warn").textContent(), "Priority 8");
    // a write that went through gets no chip in the log
    assert.equal(await page.locator(".row.log .chip").count(), 0);
    const [log] = await Promise.all([page.waitForEvent("download"), page.click("a[href^='/api/log.csv']")]);
    assert.equal(log.suggestedFilename(), "log.csv");
    assert.match(fs.readFileSync(await log.path(), "utf8"), /;1001;Analog output;1;8;write;42\.5;\r\n/);
    await page.click("#next");
    await page.waitForFunction(() => !document.querySelector("button[data-act=release-held]"));
    assert.equal(await page.$(".status .chips .chip"), null);
  },
};
