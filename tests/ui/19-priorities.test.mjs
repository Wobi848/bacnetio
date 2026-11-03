import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// All 16 priorities show at once. A write that would not act, because a higher priority holds the
// value, asks first; so does releasing a slot that does not win. A typed value survives redraws.
export default {
  name: "Priority array: 16 slots, warning before a write that would not act",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".slots");
    assert.equal(await page.$$eval(".slots .slot", (slots) => slots.length), 16);
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    // the live value redraws every 2 s and must leave what is being typed alone
    await page.fill("#new-value", "77");
    await page.waitForTimeout(2500);
    assert.equal(await page.inputValue("#new-value"), "77");
    // priority 5 holds a value
    await api("devices/1001/objects/1/1/write", "POST", { value: "45", priority: 5 });
    await page.reload();
    await page.waitForSelector(".row[data-point]", { timeout: 30000 });
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".slots .slot.win");
    assert.match(await page.textContent(".stack-now"), /Priority 5 wins/);
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "60");
    await page.click(".slots .slot:nth-child(8)");
    assert.equal(await page.inputValue("#priority"), "8");
    await page.click("button[data-act=write]");
    assert.match(await page.textContent(".ask"), /Priority 5 holds .* higher than 8/);
    await page.click("button[data-act=confirm-no]");
    assert.equal(await page.$(".slots .slot:nth-child(8).held"), null, "cancel writes nothing");
    await page.click("button[data-act=write]");
    await page.click("button[data-act=confirm-yes]");
    await page.waitForSelector(".slots .slot:nth-child(8).mine");
    await page.click("button[data-act=release][data-priority='8']");
    assert.match(await page.textContent(".ask"), /releasing 8 changes nothing/);
    await page.click("button[data-act=confirm-yes]");
    await page.waitForFunction(() => !document.querySelector(".slots .slot:nth-child(8).held"));
    await api("devices/1001/objects/1/1/release", "POST", { priority: 5 });
  },
};
