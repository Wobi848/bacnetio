import { fileURLToPath } from "node:url";
import { assert, open, openDevice, searchInUi, showSite } from "./helpers.mjs";

// A plugin is installed from its file, runs in a sandbox, asks only what plugin API 1 answers,
// marks a device and a point, and goes again when switched off and removed.
const PLUGIN = fileURLToPath(new URL("./fixtures/test.bacnetio-plugin.html", import.meta.url));

export default {
  name: "Plugins: install, ask, mark, switch off, remove",
  async run({ page, base }) {
    await open(page, base, { area: "workspace" });
    await searchInUi(page);
    await page.click("a.settings-link");
    assert.match(await page.textContent(".card:has(#plugin-file)"), /No extension installed/);
    await page.setInputFiles("#plugin-file", PLUGIN);
    await page.waitForSelector(".plugins .plugin:has-text('Test plugin')");

    await page.click(".tabs a:has-text('Test plugin')");
    const frame = page.frameLocator("iframe.plugin-frame");
    await frame.locator("#done").waitFor({ timeout: 30000 });
    assert.equal(await frame.locator("#failed").count(), 0);
    assert.equal(await frame.locator("#language").textContent(), "en");
    assert.equal(await frame.locator("#accent").textContent(), "given");
    assert.ok(Number(await frame.locator("#devices").textContent()) > 1);
    assert.ok(Number(await frame.locator("#objects").textContent()) > 5);
    assert.equal(await frame.locator("#properties").textContent(), "read");
    assert.match(await frame.locator("#kept").textContent(), /^\{"seen":\d+\}$/);
    assert.equal(await frame.locator("#marked").textContent(), "2");
    assert.equal(await frame.locator("#named").textContent(), "2");
    // the sandbox: no way to the server, and no question that writes
    assert.equal(await frame.locator("#server").textContent(), "blocked");
    assert.equal(await frame.locator("#write").textContent(), "unknown question write");
    assert.ok(await page.isHidden("#work"));

    // a reload on the plugin's page starts it at once; it still gets the devices, not an empty list
    await page.reload();
    await page.frameLocator("iframe.plugin-frame").locator("#done").waitFor({ timeout: 30000 });
    assert.ok(Number(await page.frameLocator("iframe.plugin-frame").locator("#devices").textContent()) > 1);

    // the plugin hands the user a file
    const [download] = await Promise.all([page.waitForEvent("download"), page.frameLocator("iframe.plugin-frame").locator("#report").click()]);
    assert.equal(download.suggestedFilename(), "test-report.csv");

    // started with bacnetio: on another page after a reload the marks are there without opening it
    await page.click(".tabs a[data-area=workspace]");
    await page.reload();
    // the marks show in bacnetio's own lists, with the plugin's label and state, or its name
    const device = page.locator(".node[data-act=tree-device]:has-text('AHU-01') .chip.mark.ok");
    await device.waitFor();
    assert.equal(await device.textContent(), "Test ✓");
    await openDevice(page, "AHU-01");
    const row = page.locator(".row[data-point]:has-text('Outdoor air temp') .chip.mark.bad");
    assert.equal(await row.textContent(), "Test plugin");
    assert.equal(await row.getAttribute("title"), "Test plugin: Object note");

    // the priority grid takes the plugin's names before BACnet's own, and says whose they are
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".slots .slot");
    assert.equal(await page.textContent(".slots .slot:nth-child(8) .who"), "Manual test");
    assert.equal(await page.textContent(".slots .slot:nth-child(9) .who"), "Free test");
    assert.equal(await page.getAttribute(".slots .slot:nth-child(8)", "title"), "Manual test (Test plugin)");

    // a search after the plugin started reaches its open page without a reload
    await page.click(".tabs a:has-text('Test plugin')");
    await page.frameLocator("iframe.plugin-frame").locator("#done").waitFor({ timeout: 30000 });
    await showSite(page);
    await searchInUi(page);
    await page.click(".tabs a:has-text('Test plugin')");
    await page.frameLocator("iframe.plugin-frame").locator("#heard").waitFor();

    // the same plugin again: asked before it is replaced
    await page.click("a.settings-link");
    await page.setInputFiles("#plugin-file", PLUGIN);
    await page.waitForSelector("text=Test plugin is installed already");
    await page.click("button[data-act=plugin-replace-yes]");
    await page.waitForSelector("button[data-act=plugin-replace-yes]", { state: "detached" });

    // switched off: no entry, no frame, no marks
    await page.click("button[data-act=plugin-switch][data-on=false]");
    await page.waitForSelector(".tabs a:has-text('Test plugin')", { state: "detached" });
    assert.equal(await page.locator("iframe.plugin-frame").count(), 0);
    await page.click(".tabs a[data-area=workspace]");
    assert.equal(await page.locator(".chip.mark").count(), 0);

    await page.click("a.settings-link");
    // removing asks in a dialog, the focus in it
    await page.click("button[data-act=plugin-remove]");
    await page.waitForSelector(".modal .modal-box:has-text('Remove Test plugin')");
    assert.ok(await page.evaluate(() => document.querySelector(".modal-box").contains(document.activeElement)));
    await page.click("button[data-act=plugin-remove-yes]");
    await page.waitForSelector("text=No extension installed");
  },
};
