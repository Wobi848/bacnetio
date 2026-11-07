import { assert, open, openDevice, searchInUi, showSite } from "./helpers.mjs";

// Device list, point list and recording download as CSV.
async function download(page, selector) {
  // the device list has its export menu, the point list keeps its exports under "More …"
  if (!(await page.$(selector))) await page.click("button[data-act=export-menu], button[data-act=more-menu]");
  const [file] = await Promise.all([page.waitForEvent("download"), page.click(selector)]);
  const stream = await file.createReadStream();
  let text = "";
  for await (const chunk of stream) text += chunk;
  return { name: file.suggestedFilename(), text };
}

export default {
  name: "Exports: devices, points, recording as CSV",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    const devices = await download(page, ".toolbar a[href='/api/devices.csv']");
    assert.match(devices.text, /AHU-01/);
    assert.match(devices.text, /DDC4000_#21/);
    await openDevice(page, "AHU-01");
    const points = await download(page, ".toolbar a[href$='points.csv']");
    assert.equal(points.name, "device-1001.csv");
    assert.match(points.text, /Heating valve/);
    const ede = await download(page, ".toolbar a[href$='points.ede.csv']");
    assert.equal(ede.name, "device-1001-ede.csv");
    assert.match(ede.text, /^#Engineering-Data-Exchange/);
    assert.match(ede.text, /1001_Heating valve;1001;Heating valve;1;1;/);
    const states = await download(page, ".toolbar a[href$='points.ede-states.csv']");
    assert.equal(states.name, "device-1001-ede-states.csv");
    assert.match(states.text, /^#Reference Number;/);
    assert.match(ede.text, /Operating mode;19;\d+;[^\r\n]*;Y;;;;\d+;/);
    // the priority that wins now and every set slot, read from the device
    await api("devices/1001/objects/1/1/write", "POST", { value: "40", priority: 8 });
    const held = await download(page, ".toolbar a[href$='points-priorities.csv']");
    await api("devices/1001/objects/1/1/release", "POST", { priority: 8 });
    assert.equal(held.name, "device-1001-priorities.csv");
    assert.match(held.text, /Winning priority;Set priorities/);
    assert.match(held.text, /Heating valve;.*;8;8=40/);
    await page.click(".row[data-point]:has-text('Outdoor air temp')");
    await page.click(".detail button[data-act=rec-add]");
    await page.keyboard.press("4");
    await page.click("#next");
    await page.waitForFunction(() => /\d/.test(document.querySelector(".stats tbody td")?.textContent ?? ""), null, { timeout: 15000 });
    const recording = await download(page, ".toolbar a[href*='recording.csv']");
    assert.match(recording.text, /Outdoor air temp/);
    await page.click("button[data-act=rec-stop]");
    // the whole site: the device not read yet is read first
    await showSite(page);
    const site = await download(page, "button[data-act=site-ede]");
    assert.equal(site.name, "site-ede.csv");
    assert.match(site.text, /\r?\n1001_Heating valve;1001;/);
    assert.match(site.text, /\r?\n21_[^;]*;21;/);
  },
};
