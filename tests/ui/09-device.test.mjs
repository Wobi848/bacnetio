import { assert, open, searchInUi } from "./helpers.mjs";

// The chosen device's facts stand beside the list; setting its clock needs the unlock.
export default {
  name: "Device details and clock",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await page.click(".row[data-device]:has-text('AHU-01')");
    await page.waitForFunction(() => document.querySelector(".detail .facts")?.textContent.includes("Firmware"));
    // the simulators with objects refuse GetEventInformation, but know GetAlarmSummary: an empty list
    // (a refusal in words is tested in tests/alarms.rs)
    await page.waitForFunction(() => /No active alarms|does not give its active alarms/.test(document.querySelector(".detail").textContent));
    await page.click("button[data-act=toggle-commands]");
    await page.click("button[data-act=ask-clock]");
    await page.click("text=Allow changes for this session");
    await page.click("button[data-act=set-clock]");
    await page.waitForFunction(() => /sent to the device/.test(document.getElementById("toast").textContent));
    // like every write, setting the clock is logged
    await page.keyboard.press("5");
    await page.waitForFunction(() => /sent the time \d{4}-\d\d-\d\d/.test(document.querySelector(".rows")?.textContent));
    assert.doesNotMatch(await page.textContent(".rows"), /AHU-01 · /);
    assert.match(await page.textContent(".rows"), /AHU-01\s*sent the time/);

  },
};
