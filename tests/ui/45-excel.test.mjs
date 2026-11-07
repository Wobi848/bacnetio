import { assert, open, openDevice, searchInUi, showSite } from "./helpers.mjs";

// The Excel files beside the CSV (docs/concept-excel.md): the device list, a device's points and
// the whole site download as .xlsx with the heads in the page's language.
async function download(page, menu, selector) {
  if (!(await page.$(selector))) await page.click(menu);
  const [file] = await Promise.all([page.waitForEvent("download"), page.click(selector)]);
  const chunks = [];
  for await (const chunk of await file.createReadStream()) chunks.push(chunk);
  return { name: file.suggestedFilename(), bytes: Buffer.concat(chunks), url: file.url() };
}

export default {
  name: "Excel files beside the CSV",
  async run({ page, base }) {
    await open(page, base, { language: "de" });
    await searchInUi(page);
    await showSite(page);
    const devices = await download(page, "button[data-act=export-menu]", "a[href^='/api/devices.xlsx']");
    assert.equal(devices.name, "devices.xlsx");
    assert.equal(devices.bytes.subarray(0, 2).toString(), "PK", "an xlsx file is a zip");
    const heads = JSON.parse(new URL(devices.url).searchParams.get("heads"));
    assert.equal(heads["Device ID"], "Geräte-ID", "the heads in the page's language");
    await openDevice(page, "AHU-01");
    const points = await download(page, "button[data-act=more-menu]", "a[href^='/api/devices/1001/points.xlsx']");
    assert.equal(points.name, "device-1001.xlsx");
    assert.match(new URL(points.url).searchParams.get("title"), /AHU-01/);
    // the whole site reads the devices not read yet first
    await showSite(page);
    await page.click("button[data-act=export-menu]");
    const [file] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }), page.click("button[data-act=site-excel]")]);
    assert.equal(file.suggestedFilename(), "site.xlsx");
  },
};
