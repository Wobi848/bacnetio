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
    await searchInUi(page, 3);
    await page.click("a.settings-link");
    await page.setInputFiles("#plugin-file", path.join(out, "kbob.bacnetio-plugin.html"));
    await page.click(".tabs a:has-text('KBOB')");
    const plugin = page.frameLocator("iframe.plugin-frame");
    // the AKS names of the guide's example on top of the rules
    await plugin.locator("details.chooser summary").click();
    assert.match(await plugin.locator("details.chooser summary").textContent(), /10 of 11/);
    await plugin.locator("input[data-act=rule][data-rule=aks]").check();
    assert.match(await plugin.locator("details.chooser summary").textContent(), /11 of 11/);
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
