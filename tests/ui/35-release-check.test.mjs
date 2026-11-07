import { assert, open, searchByApi } from "./helpers.mjs";

// Days after a write another system may own the slot. Releasing from the list of held values then
// takes nothing away: bacnetio reads the slot first and says why it did not release.
const VALVE = "devices/1001/objects/1/1";

export default {
  name: "Release from the held list only while the slot is still ours",
  async run({ page, base, api, another }) {
    await searchByApi(api);
    await api(`${VALVE}/write`, "POST", { value: "30", priority: 9 });
    const other = await another();
    await searchByApi(other.api);
    await other.api(`${VALVE}/write`, "POST", { value: "60", priority: 9 });
    // a device may show a write a moment late; bacnetio counts a slot as taken over after 10 s
    await new Promise((done) => setTimeout(done, 10500));

    await open(page, base, { area: "log" });
    await page.waitForSelector(".row.held button[data-act=release-held]");
    assert.match(await page.textContent(".row.held .name"), /held since \d\d:\d\d/);
    await page.click(".row.held button[data-act=release-held]");
    await page.waitForFunction(() => /priority 9 holds 60/i.test(document.querySelector("#toast")?.textContent ?? ""));
    assert.match(await page.textContent("#toast"), /not what bacnetio wrote: another system wrote over it, so it was not released/);
    // no longer counted as ours, and the other system's value is still there
    await page.waitForSelector(".row.held button[data-act=release-held]", { state: "detached" });
    const valve = await api(VALVE);
    assert.equal(valve.priority_array.find((slot) => slot.priority === 9).value, "60 %");

    await other.api(`${VALVE}/release`, "POST", { priority: 9 });
  },
};
