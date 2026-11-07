import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Changing many points at once (docs/concept-toolbox.md part 5), against the simulated AHU-01:
// chosen by the filter, behind the unlock and one more question, each written and held, then set
// back the same way.
async function changeOutOfService(page, value) {
  await page.click("[data-act=many-open]");
  await page.waitForSelector(".many-panel");
  await page.selectOption("#many-kind", "outOfService");
  await page.selectOption("#many-value", value);
  if (await page.$(".many-panel [data-act=unlock]")) await page.click(".many-panel [data-act=unlock]");
  await page.click("[data-act=many-ask]");
  await page.waitForSelector(".many-panel .ask [data-act=many-write]");
  assert.match(await page.textContent(".many-panel .ask"), /Out of service = (Yes|No) to 9 points of AHU-01/);
  await page.click("[data-act=many-write]");
  await page.waitForSelector(".many-panel [data-act=many-close]", { timeout: 30000 });
  assert.match(await page.textContent(".many-panel"), /9 written, 0 not written/);
  await page.click("[data-act=many-close]");
}

export default {
  name: "Change many points at once",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click("[data-act=select-toggle]");
    await page.fill("#filter", "Room temp 2.0");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point]").length === 9);
    // a group's box chooses the group and gives it back
    await page.click(".row.group [data-act=pick-group]");
    assert.match(await page.textContent(".pick-bar"), /9 selected/);
    await page.click(".row.group [data-act=pick-group]");
    assert.match(await page.textContent(".pick-bar"), /0 selected/);
    await page.click("[data-act=pick-shown]");
    assert.match(await page.textContent(".pick-bar"), /9 selected/);
    // a point the change does not suit is shown and left out
    await page.click("[data-act=many-open]");
    await page.selectOption("#many-kind", "value");
    await page.fill("#many-value", "30");
    assert.equal(await page.locator(".many-table tr.left-out").count(), 9, "inputs have no commanded value");
    assert.equal(await page.locator(".many-panel [data-act=many-ask]").count(), 0, "the unlock comes first");
    await page.click(".many-panel [data-act=unlock]");
    assert.equal(await page.isDisabled("[data-act=many-ask]"), true, "nothing to write");
    await page.click("[data-act=many-close]");
    await changeOutOfService(page, "true");
    const held = await api("held");
    assert.equal(held.filter((entry) => entry.device === 1001).length, 9, "each one held, for the leaving check");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point] .chip.warn").length >= 9);
    await changeOutOfService(page, "false");
    assert.equal((await api("held")).filter((entry) => entry.device === 1001).length, 0);
  },
};
