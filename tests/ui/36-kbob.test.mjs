import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SIM_PORT, assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The KBOB plugin, built as it is released: checks every device of the search by reading only,
// says per rule what is met, marks met and not met in bacnetio's lists and keeps it over a reload.
// Beside the plant a third simulator meets the profile AS-CH in full.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

export default {
  name: "KBOB plugin: check all devices, results, marks, report",
  async run({ page, base }) {
    const out = fs.mkdtempSync(path.join(os.tmpdir(), "bacnetio-plugins-"));
    execFileSync(process.execPath, [path.join(root, "plugins/build.mjs"), out]);
    const exe = process.platform === "win32" ? ".exe" : "";
    const meetsAll = spawn(path.join(root, `target/debug/examples/simulated_kbob${exe}`), ["127.0.0.4", String(SIM_PORT)], { stdio: "ignore" });
    try {
      await this.check(page, base, out);
    } finally {
      meetsAll.kill();
      fs.rmSync(out, { recursive: true, force: true });
    }
  },

  async check(page, base, out) {
    await open(page, base, { area: "workspace" });
    await page.click("a.settings-link");
    await page.setInputFiles("#plugin-file", path.join(out, "kbob.bacnetio-plugin.html"));
    await page.click(".tabs a:has-text('KBOB')");
    const plugin = page.frameLocator("iframe.plugin-frame");
    await plugin.locator("#app p.muted").waitFor();
    // a search after the plugin started reaches its open page without a reload
    await page.click(".tabs a[data-area=workspace]");
    await searchInUi(page, 3);
    await page.click(".tabs a:has-text('KBOB')");
    await plugin.locator("tbody tr:has-text('3022_LU1_L01_D001')").waitFor();
    // the AKS names of the guide's example on top of the rules
    await plugin.locator("details.chooser summary").click();
    assert.match(await plugin.locator("details.chooser summary").textContent(), /11 of 12/);
    await plugin.locator("input[data-act=rule][data-rule=aks]").check();
    // the choice is saved first, then the count is drawn anew
    await plugin.locator("details.chooser summary:has-text('12 of 12')").waitFor();
    await plugin.locator("button[data-act=check-all]").click();
    // both devices checked, one after the other; meanwhile the check can be stopped
    await plugin.locator("button[data-act=stop]").waitFor();
    await plugin.locator("button[data-act=check-all]").waitFor({ timeout: 120000 });
    const results = await plugin.locator("tbody tr td:nth-child(3)").allTextContents();
    assert.equal(results.length, 3, results.join(" | "));
    assert.match(await plugin.locator("tr:has-text('3022_LU1_L01_D001') td:nth-child(3)").textContent(), /^all rules met/);
    assert.match(await plugin.locator("tr:has-text('AHU-01') td:nth-child(3)").textContent(), /rules not met/);

    // the simulated AHU lacks properties the profile asks of an analog input
    await plugin.locator("tr:has-text('AHU-01') button[data-act=toggle]").click();
    const properties = plugin.locator("li.rule:has-text('Each object has the properties')");
    assert.match(await properties.textContent(), /not met/);
    assert.match(await properties.textContent(), /AI 2.*device-type/s);
    assert.match(await plugin.locator("li.rule:has-text('AKS pattern')").textContent(), /not met.*Outdoor air temp/s);

    // the marks in bacnetio's own lists
    await page.click(".tabs a[data-area=workspace]");
    await openDevice(page, "AHU-01");
    const mark = page.locator(".row[data-point]:has-text('Outdoor air temp') .chip.mark.bad");
    assert.equal(await mark.textContent(), "KBOB ✗");
    assert.match(await mark.getAttribute("title"), /^KBOB: missing: .*device-type.* \(5\.3, C1\.2\)/);
    assert.equal(await page.textContent(".node[data-act=tree-device]:has-text('3022_LU1_L01_D001') .chip.mark.ok"), "KBOB ✓");

    // the priority grid names each priority after KBOB table 10, and says the names come from it
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".slots .slot");
    assert.equal(await page.textContent(".slots .slot:nth-child(8) .who"), "Manual from MBE");
    assert.equal(await page.textContent(".slots .slot:nth-child(9) .who"), "free (owner)");
    assert.equal(await page.getAttribute(".slots .slot:nth-child(16)", "title"), "Automatic mode (KBOB)");

    // a device read but without a plant view or path names says so in the plant tree, whole
    await openDevice(page, "3022_LU1_L01_D001");
    await page.click("button[data-act=tree-view][data-view=plant]");
    const note = page.locator(".tree .tree-note");
    assert.match(await note.textContent(), /^No plant parts: .* All its objects are under By type\.$/);
    assert.ok(await note.evaluate((element) => element.scrollWidth <= element.clientWidth), "not cut off");
    await page.click("button[data-act=tree-view][data-view=type]");

    // started with bacnetio: after a reload the marks are there without opening the plugin
    await page.reload();
    await page.waitForSelector(".node[data-act=tree-device]:has-text('AHU-01') .chip.mark.bad");

    // kept over a reload, and the report as CSV
    await page.click(".tabs a:has-text('KBOB')");
    await page.reload();
    await plugin.locator("tr:has-text('AHU-01') td:nth-child(3):has-text('rules not met')").waitFor();
    const [download] = await Promise.all([page.waitForEvent("download"), plugin.locator("button[data-act=report]").click()]);
    assert.equal(download.suggestedFilename(), "kbob-report.csv");
    const report = fs.readFileSync(await download.path(), "utf8");
    assert.ok(report.startsWith("\uFEFF"), "Excel reads the umlauts");
    assert.equal(report.split("\r\n")[0], '\uFEFF"Device";"Name";"Profile";"Checked";"Rule";"Source (KBOB)";"Result";"Object";"Finding"');
    assert.match(report, /"1001";"AHU-01";"AS-CH";[^\n]*"Each object has the properties its type requires";"5\.3, C1\.2";"not met";"AI 2"/);
  },
};
