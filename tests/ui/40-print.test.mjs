import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The print view (docs/concept-toolbox.md part 1): the device's points as a page, with the list's
// filter, without bacnetio around it; on paper without its buttons; Ctrl+P in the list opens it.
export default {
  name: "Print view of a device's points",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    const all = await page.locator(".row[data-point]").count();
    await page.fill("#filter", "valve");
    await page.waitForFunction((count) => document.querySelectorAll(".row[data-point]").length < count, all);
    const filtered = await page.locator(".row[data-point]").count();
    await page.click("[data-act=more-menu]");
    await page.click("[data-act=print-open]");
    await page.waitForSelector(".print-page");
    assert.equal(await page.isVisible("#side"), false, "no sidebar on the page to print");
    assert.equal(await page.locator(".print-table tbody tr:not(.print-group)").count(), filtered, "the rows of the filtered list");
    assert.match(await page.textContent(".print-facts"), /"valve": \d+ of \d+ points/);
    assert.match(await page.textContent(".print-facts"), /AHU-01/);
    // on paper only the page: the buttons stay on screen
    await page.emulateMedia({ media: "print" });
    assert.equal(await page.isVisible(".print-acts"), false);
    assert.equal(await page.isVisible(".print-table"), true);
    await page.emulateMedia({ media: "screen" });
    await page.click("[data-act=print-close]");
    await page.waitForSelector("#filter");
    assert.equal(await page.isVisible("#side"), true);
    // Ctrl+P in the list prints the print view, not the screen
    await page.evaluate(() => { window.printed = 0; window.print = () => { window.printed += 1; }; });
    await page.click(".rows");
    await page.keyboard.press("Control+p");
    await page.waitForSelector(".print-page");
    assert.equal(await page.evaluate(() => window.printed), 1);
    await page.keyboard.press("Escape");
    await page.waitForSelector("#filter");
  },
};
