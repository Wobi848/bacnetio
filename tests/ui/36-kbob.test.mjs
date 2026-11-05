import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The KBOB plugin, built as it is released: checks every device of the search by reading only,
// says per rule what is met, marks what is not in bacnetio's lists and keeps it over a reload.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

export default {
  name: "KBOB plugin: check all devices, results, marks, report",
  async run({ page, base }) {
    const out = fs.mkdtempSync(path.join(os.tmpdir(), "bacnetio-plugins-"));
    execFileSync(process.execPath, [path.join(root, "plugins/build.mjs"), out]);
    await open(page, base, { area: "workspace" });
    await searchInUi(page);
    await page.click("a.settings-link");
    await page.setInputFiles("#plugin-file", path.join(out, "kbob.bacnetio-plugin.html"));
    await page.click(".tabs a:has-text('KBOB')");
    const plugin = page.frameLocator("iframe.plugin-frame");
    await plugin.locator("button[data-act=check-all]").click();
    // both devices checked, one after the other; meanwhile the check can be stopped
    await plugin.locator("button[data-act=stop]").waitFor();
    await plugin.locator("button[data-act=check-all]").waitFor({ timeout: 120000 });
    const results = await plugin.locator("tbody tr td:nth-child(3)").allTextContents();
    assert.equal(results.length, 2, results.join(" | "));
    for (const result of results) assert.match(result, /rules not met|all rules met/, result);

    // the simulated AHU lacks properties the profile asks of an analog input
    await plugin.locator("tr:has-text('AHU-01') button[data-act=toggle]").click();
    const properties = plugin.locator("li.rule:has-text('Each object has the properties')");
    assert.match(await properties.textContent(), /not met/);
    assert.match(await properties.textContent(), /AI 2.*device-type/s);

    // the marks in bacnetio's own lists
    await page.click(".tabs a[data-area=workspace]");
    await openDevice(page, "AHU-01");
    const mark = page.locator(".row[data-point]:has-text('Outdoor air temp') .chip.mark");
    assert.equal(await mark.textContent(), "KBOB");
    assert.match(await mark.getAttribute("title"), /KBOB: missing: .*device-type.* \(5\.3, C1\.2\)/);

    // kept over a reload, and the report as CSV
    await page.click(".tabs a:has-text('KBOB')");
    await page.reload();
    await plugin.locator("tr:has-text('AHU-01') td:nth-child(3):has-text('rules not met')").waitFor();
    const [download] = await Promise.all([page.waitForEvent("download"), plugin.locator("button[data-act=report]").click()]);
    assert.equal(download.suggestedFilename(), "kbob-report.csv");
    const report = fs.readFileSync(await download.path(), "utf8");
    assert.match(report.split("\r\n")[0], /"device";"name";"profile";"checked";"rule";"source";"result";"object";"finding"/);
    assert.match(report, /"1001";"AHU-01";"AS-CH";[^\n]*"properties";"5\.3, C1\.2";"not met";"AI 2"/);
    fs.rmSync(out, { recursive: true, force: true });
  },
};
