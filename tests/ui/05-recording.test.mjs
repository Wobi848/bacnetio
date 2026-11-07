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
    // before the start the page says so, not that it waits for a value; the head says what it does
    assert.match(await page.textContent(".strips"), /is not running: press “Start recording” above/);
    assert.match(await page.textContent(".pagehead .lead"), /^Chosen values every 10 s, as charts and CSV\./);
    assert.match(await page.textContent(".toolbar .rec-state"), /^Stopped · 1 point · 0 values$/);
    assert.equal(await page.textContent(".toolbar a[href*='recording.csv']"), "Export CSV");
    // recorded by the device's reports instead of polling
    await page.click("button[data-act=rec-mode][data-mode=cov]");
    await page.waitForSelector("button[data-act=rec-mode][data-mode=cov][aria-pressed=true]");
    await page.click("#next");
    // while it runs nothing is filled: stopping is a tool in the head
    await page.waitForSelector(".pagehead + .next-acts button.quiet[data-act=rec-stop], .next-acts button.quiet[data-act=rec-stop]");
    assert.equal(await page.$("#next"), null);
    assert.equal(await page.$(".strips"), null);
    assert.match(await page.textContent(".status .chips"), /Recording/);
    // the simulated temperatures move, so wait for any measured value, not a given one
    await page.waitForFunction(() => /\d °C/.test(document.querySelector(".stats tbody")?.textContent ?? ""), null, { timeout: 15000 });
    assert.match(await page.textContent(".stats tbody"), /Measured \d/);
    assert.ok(await page.$(".chart .mark"), "each measurement has its dot");
    // a second value of the same unit shares the chart; another unit gets its own
    assert.equal(await page.$$eval(".chart.compare", (charts) => charts.length), 1);
    assert.match(await page.textContent(".stats thead"), /Lowest/);
    await page.click("button[data-act=rec-window][data-minutes='5']");
    await page.click("button[data-act=rec-view][data-view=cards]");
    await page.waitForSelector(".card .series-head");
    // a chart is drawn for its width, 220 px high, and says when and how often it was measured
    await page.waitForFunction(() => { const svg = document.querySelector(".card .chart svg"); return svg && Math.abs(svg.getBoundingClientRect().height - 220) < 2 && Math.abs(svg.viewBox.baseVal.width - svg.getBoundingClientRect().width) < 6; });
    assert.match(await page.textContent(".card .measured"), /^Measured .* · on change$/);
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
    await page.click("button[data-act=rec-stop]");
    await page.waitForFunction(() => document.querySelector("#next")?.textContent === "Start recording");
    await page.click("button[data-act=rec-clear]");
    await page.waitForFunction(() => /· 0 values$/.test(document.querySelector(".toolbar .rec-state")?.textContent ?? ""));
    await page.click("button[data-act=rec-remove]");
    await page.waitForSelector("text=Nothing recorded yet");
  },
};
