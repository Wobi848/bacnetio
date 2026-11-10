import path from "node:path";
import { assert, open, openDevice } from "./helpers.mjs";

// BACnet/SC, step 2 (docs/concept-sc.md): the site's files chosen and checked in words, a key with
// its password, the connection to the hub with a search, a value held on an SC device, leaving the
// SC network and the log. Against simulated_sc, which the runner starts.
export default {
  name: "BACnet/SC: choose the files, connect to the hub, find its device, leave",
  async run({ page, base }) {
    const dir = process.env.BACNETIO_TEST_SC_DIR;
    if (!dir) return;
    const file = (name) => path.join(dir, name);
    const chip = (line) => page.textContent(`dd.sc-line:has(#sc-file-${line}) .chip`);
    await open(page, base);
    await page.click("#search-settings");
    await page.click("#link-sc");
    await page.waitForSelector("#sc-hub");
    assert.match(await chip("ca"), /Not chosen yet/);
    assert.ok(await page.isDisabled("[data-act=sc-search]"));

    await page.fill("#sc-hub", process.env.BACNETIO_TEST_SC_HUB);
    await page.setInputFiles("#sc-file-ca", file("ca.pem"));
    await page.waitForFunction(() => /Valid until/.test(document.querySelector("dd.sc-line:has(#sc-file-ca) .chip")?.textContent ?? ""));
    await page.setInputFiles("#sc-file-own", file("bacnetio.pem"));
    await page.waitForFunction(() => /Valid until/.test(document.querySelector("dd.sc-line:has(#sc-file-own) .chip")?.textContent ?? ""));
    // a certificate where the key belongs says so
    await page.setInputFiles("#sc-file-key", file("ca.pem"));
    await page.waitForFunction(() => /No key in this file/.test(document.body.textContent));
    assert.ok(await page.isDisabled("[data-act=sc-search]"));
    // the key with a password asks for it, refuses a wrong one and opens with the right one
    await page.setInputFiles("#sc-file-key", file("bacnetio-locked.key"));
    await page.waitForSelector("#sc-password-key");
    assert.match(await chip("key"), /Needs its password/);
    await page.fill("#sc-password-key", "wrong");
    await page.click("[data-act=sc-open][data-line=key]");
    await page.waitForFunction(() => /Wrong password/.test(document.body.textContent));
    await page.fill("#sc-password-key", "demo");
    await page.click("[data-act=sc-open][data-line=key]");
    await page.waitForFunction(() => /Matches the certificate/.test(document.body.textContent));
    assert.match(await page.textContent("dd.sc-line:has(#sc-file-key)"), /bacnetio-locked\.key/);
    // a key that came with a password is kept only when asked
    assert.equal(await page.isChecked("#sc-remember"), false);

    await page.click("[data-act=sc-search]");
    // the name is read right after the device answered
    await page.waitForSelector(".row[data-device='5001']:has-text('SC-AHU')", { timeout: 30000 });
    const row = await page.textContent(".row[data-device='5001']");
    assert.match(row, /SC-AHU/);
    assert.match(row, /SC 22:00:00:00:00:01/);
    assert.match(await page.textContent(".side-trunk"), /On the BACnet\/SC network at localhost:\d+/);
    // no device by IP on an SC network
    assert.equal(await page.locator("[data-act=locate-menu]").count(), 0);

    // points read and a value written through the hub, as on IP
    await openDevice(page, "SC-AHU");
    await page.waitForSelector(".row[data-point]:has-text('Supply air temp')");
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".detail .big");
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "55");
    await page.click("button[data-act=write]");
    await page.waitForSelector(".slots .slot.mine", { timeout: 15000 });
    // leaving the SC network asks first, as leaving bacnetio does
    await page.click("[data-act=sc-leave]");
    assert.match(await page.textContent(".side-trunk"), /You still hold a value/);
    await page.click(".side-trunk [data-act=release-all]");
    await page.waitForFunction(() => !/held/.test(document.querySelector(".status .chips")?.textContent ?? ""), null, { timeout: 15000 });
    await page.click("[data-act=sc-leave]");
    await page.waitForFunction(() => !document.querySelector(".side-trunk"));

    await page.goto(`${base}/#/log`);
    await page.waitForFunction(() => /Left the hub/.test(document.body.textContent));
    const log = await page.textContent("main");
    assert.match(log, /BACnet\/SC network/);
    assert.match(log, /Connected to the hub wss:\/\/localhost:\d+\//);

    // after a reload the hub and the files are offered again
    await page.reload();
    await page.click("#search-settings");
    await page.click("#link-sc");
    await page.waitForFunction(() => /Matches the certificate/.test(document.body.textContent));
    assert.equal(await page.inputValue("#sc-hub"), process.env.BACNETIO_TEST_SC_HUB);
    assert.ok(!(await page.isDisabled("[data-act=sc-search]")));
  },
};
