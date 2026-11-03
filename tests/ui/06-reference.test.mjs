import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A saved reference adds the "Changed" view; nothing changed right after saving.
export default {
  name: "Reference: save, then show changes",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click("button[data-act=more-menu]");
    await page.click("button[data-act=ref-save]");
    await page.waitForSelector(".seg [data-view=changed]");
    await page.click(".seg [data-view=changed]");
    assert.match(await page.textContent(".rows"), /Nothing changed since the reference/);
  },
};
