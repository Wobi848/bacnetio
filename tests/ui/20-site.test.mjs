import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// The site as a whole: problems across devices, the search, the watch list, and leaving.
export default {
  name: "Site problems, search, watch list, leaving",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    // a site without a name shows its network as its name, once: no placeholder, no naming button
    // the kept sites come in after the search, a moment later in some browsers
    await page.waitForFunction(() => document.querySelector(".side .site-group .site-now")?.textContent === "127.0.0.0/29");
    assert.equal(await page.$(".side .site-group .site-net"), null, "the network is said once");
    assert.match(await page.textContent(".side .site-group .side-detail"), /2 devices found/);
    assert.doesNotMatch(await page.textContent(".status"), /127\.0\.0\./);
    // the site is a select field; the search settings a framed icon with its name for the ear
    assert.equal(await page.getAttribute(".side .site-switch", "aria-haspopup"), "listbox");
    assert.equal(await page.getAttribute(".side button[data-act=network]", "aria-label"), "Search settings");
    assert.equal(await page.getAttribute(".side button[data-act=network]", "title"), "Search settings");
    assert.equal(await page.$(".side-card .adapter-name"), null, "the adapter is setup, in the search settings");
    // reached with the keyboard: from the site field one Tab on, Enter opens the settings
    await page.focus(".side .site-switch");
    await page.keyboard.press("Tab");
    assert.equal(await page.evaluate(() => document.activeElement.dataset.act), "network");
    await page.keyboard.press("Enter");
    // a dialog on a dimmed page: the focus goes in, Tab keeps it inside, Escape gives it back
    await page.waitForSelector(".modal .search-settings #adapter");
    assert.equal(await page.evaluate(() => document.activeElement.id), "subnet");
    for (let step = 0; step < 12; step += 1) await page.keyboard.press("Tab");
    assert.ok(await page.evaluate(() => document.querySelector(".modal-box").contains(document.activeElement)), "Tab stays in the dialog");
    await page.keyboard.press("Escape");
    await page.waitForSelector("#adapter", { state: "detached" });
    assert.equal(await page.evaluate(() => document.activeElement.dataset.act), "network");
    // the site field: the arrows move through the sites, Escape closes it and gives the focus back
    await page.click(".side .site-switch");
    assert.equal(await page.evaluate(() => document.activeElement.getAttribute("aria-checked")), "true");
    await page.keyboard.press("ArrowDown");
    assert.equal(await page.evaluate(() => document.activeElement.dataset.act), "sites-all");
    await page.keyboard.press("Escape");
    assert.equal(await page.$(".site-choices"), null);
    assert.equal(await page.evaluate(() => document.activeElement.dataset.act), "site-menu");
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
