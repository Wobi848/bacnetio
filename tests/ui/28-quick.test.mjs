import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Watching and recording straight from a row of the list, without opening the details.
export default {
  name: "Quick actions: watch and record from the list",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    const row = ".row[data-point]:has-text('Supply air temp')";
    await page.click(`${row} button[data-act=quick-watch]`);
    await page.waitForSelector(`${row} button[data-act=quick-watch][aria-pressed=true]`);
    assert.equal(await page.$(".detail .big"), null, "the details stay closed");
    await page.click(`${row} button[data-act=quick-record]`);
    await page.waitForSelector(`${row} button[data-act=quick-record][aria-pressed=true]`);
    assert.match(await page.textContent(".tabs"), /Watch list\s*1/);
    // a second click takes it out again
    await page.click(`${row} button[data-act=quick-watch]`);
    await page.waitForSelector(`${row} button[data-act=quick-watch][aria-pressed=false]`);
  },
};
