import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A point goes into the recording from its details; the recording page starts and stops it.
export default {
  name: "Recording: add a point, start, stop, remove",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Supply air temp')");
    await page.click(".detail button[data-act=rec-add]");
    await page.waitForSelector(".detail button[data-act=rec-remove-here][aria-pressed=true]");
    await page.keyboard.press("4");
    await page.waitForSelector("button[data-act=rec-view]");
    // before the start the page says so, not that it waits for a value
    assert.match(await page.textContent(".strips"), /not running yet/);
    // recorded by the device's reports instead of polling
    await page.click("button[data-act=rec-mode][data-mode=cov]");
    await page.waitForSelector("button[data-act=rec-mode][data-mode=cov][aria-pressed=true]");
    await page.click("#next");
    await page.waitForFunction(() => document.querySelector("#next")?.textContent === "Stop recording");
    assert.equal(await page.$(".strips"), null);
    assert.match(await page.textContent(".status .chips"), /Recording/);
    // the simulated temperatures move, so wait for any measured value, not a given one
    await page.waitForFunction(() => /\d °C/.test(document.querySelector(".stats tbody")?.textContent ?? ""), null, { timeout: 15000 });
    assert.match(await page.textContent(".stats tbody"), /Last measured/);
    assert.ok(await page.$(".chart .mark"), "each measurement has its dot");
    // a second value of the same unit shares the chart; another unit gets its own
    assert.equal(await page.$$eval(".chart.compare", (charts) => charts.length), 1);
    assert.match(await page.textContent(".stats thead"), /Lowest/);
    await page.click("button[data-act=rec-window][data-minutes='5']");
    await page.click("button[data-act=rec-view][data-view=cards]");
    await page.waitForSelector(".card .series-head");
    await page.click("button[data-act=rec-view][data-view=compare]");
    // drawing a span zooms in, and comes back out
    // a locator finds the chart anew: the page redraws it with every measurement
    const box = await page.locator(".chart.compare svg").boundingBox();
    await page.mouse.move(box.x + box.width * 0.2, box.y + 60);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.6, box.y + 60);
    await page.mouse.up();
    await page.waitForSelector("button[data-act=rec-zoom-out]");
    await page.click("button[data-act=rec-zoom-out]");
    await page.click("#next");
    await page.waitForFunction(() => document.querySelector("#next")?.textContent === "Start recording");
    await page.click("button[data-act=rec-clear]");
    await page.waitForFunction(() => /0 samples/.test(document.querySelector(".pagehead p")?.textContent ?? ""));
    await page.click("button[data-act=rec-remove]");
    await page.waitForSelector("text=Nothing recorded yet");
  },
};
