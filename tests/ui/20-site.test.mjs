import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The site as a whole: problems across devices, the search, the watch list, and leaving.
export default {
  name: "Site problems, search, watch list, leaving",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    // the sidebar names the network once: the open site was searched where the next search looks
    // the kept sites come in after the search, a moment later in some browsers
    await page.waitForFunction(() => /127\.0\.0\.0\/29/.test(document.querySelector(".side .site-group .site-net")?.textContent ?? ""));
    // the site is named, never by its network alone; the network is said once, not in the status bar
    assert.equal(await page.$(".side .site-group .site-now"), null, "no placeholder name");
    assert.equal(await page.textContent(".side .site-group button[data-act=site-name]"), "Give the site a name");
    assert.doesNotMatch(await page.textContent(".status"), /127\.0\.0\./);
    assert.equal(await page.textContent(".side .site-group .site-link"), "Switch site");
    assert.equal(await page.textContent(".side button[data-act=network]"), "Search settings");
    assert.equal(await page.$(".side .where"), null);
    await page.keyboard.press("2");
    await page.waitForSelector("#next");
    assert.equal(await page.textContent("#next"), "Read all devices");
    // a note bacnetio worked around may stand beside it, blue as well
    await page.waitForSelector(".strip.info:has-text('2 of 2 devices are not read yet')");
    await page.click("#next");
    await page.waitForSelector(".row.problem:has-text('Out of service')", { timeout: 60000 });
    // a table with heads like the point list, sorted by a click on one
    assert.match(await page.textContent(".row.problem.head"), /Device[\s\S]*Name[\s\S]*Value[\s\S]*State/);
    // the count of problems in the sidebar says it is about problems: in the warning colour
    assert.ok(await page.isVisible(".tabs a[data-area=problems] .n.warn"));
    await page.click(".row.problem.head button[data-key=device]");
    await page.click(".row.problem.head button[data-key=device]");
    const devices = await page.$$eval(".row.problem:not(.head) > :first-child", (cells) => cells.map((cell) => cell.textContent));
    assert.deepEqual(devices, [...devices].sort().reverse());
    // searched and narrowed to one state like the point list, and back to all at once
    const problems = await page.$$eval(".row.problem:not(.head)", (rows) => rows.length);
    await page.fill("#table-filter", "nothing like this");
    await page.waitForSelector(".rows .empty:has-text('Nothing matches the filters')");
    assert.equal(await page.evaluate(() => document.activeElement.id), "table-filter");
    await page.click("button[data-act=table-clear]");
    await page.click("button[data-act=table-view][data-view=outOfService]");
    const states = await page.$$eval(".row.problem:not(.head) .chips", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(states.length && states.every((text) => /Out of service/.test(text)), states.join(" | "));
    await page.click("button[data-act=table-clear]");
    assert.equal(await page.$$eval(".row.problem:not(.head)", (rows) => rows.length), problems);
    // a problem opens its point in the workspace
    await page.click(".row.problem:has-text('Out of service')");
    await page.waitForSelector(".detail .big");
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
    assert.match(await page.textContent(".pagehead h1"), /DDC4000/);
    // the search finds points on any device, and opens them
    await page.fill("#site-search", "heating valve");
    await page.waitForSelector(".row.problem:has-text('Heating valve')");
    assert.match(await page.textContent(".pagehead p"), /found/);
    await page.click(".row.problem:has-text('Heating valve')");
    await page.waitForFunction(() => document.querySelector(".pagehead h1").textContent === "AHU-01");
    await page.waitForSelector(".detail .big");
    assert.equal(await page.inputValue("#site-search"), "");
    // on the watch list with its value, across restarts of the page
    await page.click(".detail button[data-act=watch-add]");
    await page.click(".row[data-point]:has-text('Supply air temp') button[data-act=quick-watch]");
    await page.keyboard.press("3");
    await page.waitForFunction(() => /\d/.test(document.querySelector(".row.problem:not(.head) .value")?.textContent ?? ""));
    // the head says what a click does; the name is the data and is not cut beside an empty state
    assert.match(await page.textContent(".pagehead p"), /Choose one to open it in its device\./);
    const cut = await page.$$eval(".row.problem:not(.head) .name", (cells) => cells.filter((cell) => cell.scrollWidth > cell.clientWidth).length);
    assert.equal(cut, 0);
    await page.reload();
    await page.waitForSelector(".row.problem:has-text('Heating valve')");
    await page.fill("#table-filter", "supply");
    await page.waitForFunction(() => document.querySelectorAll(".row.problem:not(.head)").length === 1);
    await page.fill("#table-filter", "");
    // sorted by name, last first: removing the first row removes that point, not the first added
    await page.click(".row.problem.head button[data-key=name]");
    await page.click(".row.problem.head button[data-key=name]");
    assert.match(await page.textContent(".row.problem:not(.head)"), /Supply air temp/);
    await page.click(".row.problem:not(.head) button[data-act=watch-remove]");
    await page.waitForFunction(() => document.querySelectorAll(".row.problem:not(.head)").length === 1);
    assert.match(await page.textContent(".row.problem:not(.head)"), /Heating valve/);
    await page.click("button[data-act=watch-remove]");
    await page.waitForSelector(".empty h2");
    // leaving asks while values are held, and offers to release them
    await page.keyboard.press("1");
    await openDevice(page, "AHU-01");
    await page.click(".row[data-point]:has-text('Heating valve')");
    await page.waitForSelector(".detail .big");
    await page.click("text=Change value");
    await page.click("text=Allow changes for this session");
    await page.fill("#new-value", "41");
    await page.click("button[data-act=write]");
    await page.waitForSelector(".slots .slot.mine");
    await page.keyboard.press("5");
    await page.click("button[data-act=leave]");
    assert.match(await page.textContent(".modal-box"), /still hold one value/);
    await page.click("button[data-act=leave-stay]");
    assert.equal(await page.$(".modal"), null);
    await page.click("button[data-act=leave]");
    await page.click("button[data-act=leave-release]");
    await page.waitForFunction(() => /Nothing is held/.test(document.getElementById("toast").textContent));
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
  },
};
