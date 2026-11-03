import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Closing the page while values are held asks first, so nothing stays overridden by accident.
export default {
  name: "Leaving with held values warns",
  async run({ page, base, context }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "40");
    await page.click("button[data-act=write]");
    await page.waitForSelector(".slots .slot.mine");
    let warned = false;
    page.removeAllListeners("dialog");
    page.on("dialog", async (dialog) => {
      warned = dialog.type() === "beforeunload";
      await dialog.dismiss();
    });
    await page.close({ runBeforeUnload: true });
    await new Promise((done) => setTimeout(done, 500));
    assert.ok(warned, "no warning before leaving");
    // release again, from a new page, so the simulator is left as it was
    const second = await context.newPage();
    await second.goto(`${base}/#/log`);
    await second.waitForSelector("#next");
    await second.click("#next");
    await second.waitForFunction(() => !document.querySelector("button[data-act=release-held]"));
  },
};
