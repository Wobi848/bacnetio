import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The BBMD tables in the device details: read when asked for, in plain words, and never written.
// The simulators are no BBMDs, so this is the answer a device that is none gives.
export default {
  name: "BBMD tables in the device details",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    assert.match(await page.textContent(".detail"), /BBMD tables/);
    assert.match(await page.textContent(".detail"), /only reads its tables and never changes them/);
    await page.click("button[data-act=bbmd-read]");
    await page.waitForSelector(".detail h4", { timeout: 20000 });
    const text = await page.textContent(".detail");
    assert.match(text, /Broadcast distribution \(BDT\)/);
    assert.match(text, /Registered devices \(FDT\)/);
    assert.match(text, /no BBMD/);
    assert.doesNotMatch(text, /This device is a BBMD/);
    // reading the tables writes nothing: the log stays as empty as it was
    assert.deepEqual(await api("log"), []);
    // the answer comes from the API as well, with both tables missing and a reason each
    const tables = await api("devices/1001/bbmd");
    assert.equal(tables.is_bbmd, false);
    assert.equal(tables.bdt, null);
    assert.equal(tables.bdt_note.code, "no_bbmd");
    assert.equal(tables.fdt_note.code, "no_bbmd");
    await page.click("button[data-act=bbmd-read]");
    await page.waitForSelector(".detail h4");
  },
};
