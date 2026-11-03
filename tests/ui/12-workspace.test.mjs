import { assert, open, openDevice, searchInUi, showSite } from "./helpers.mjs";

const rowNames = (page) => page.$$eval(".row[data-point] .name", (cells) => cells.map((cell) => cell.textContent));

// Tree, table and details: the site, a device, its plant parts and types; old addresses lead here.
export default {
  name: "Workspace: tree, table and details",
  async run({ page, base }) {
    await open(page, base, { area: "overview" });
    assert.equal(await page.evaluate(() => location.hash), "#/overview");
    assert.equal(await page.textContent("#next"), "Find devices");
    await searchInUi(page);
    const devices = await page.$$eval(".node[data-act=tree-device] .label", (cells) => cells.map((cell) => cell.textContent));
    assert.deepEqual(devices.sort(), ["AHU-01", "DDC4000_#21"]);
    // the types are known from the search, without opening the device
    await page.click(".node:has-text('DDC4000') [data-act=tree-toggle]");
    assert.match(await page.textContent(".tree"), /Binary output/);
    assert.equal(await page.$("h1:has-text('DDC4000')"), null, "showing the types does not open the device");
    await openDevice(page, "DDC4000_#21");
    assert.equal(await page.textContent(".pagehead h1"), "DDC4000_#21");
    // a plant part from the names: the table shows only its points, the details the device's facts
    await page.click("button[data-act=tree-view][data-view=plant]");
    await page.click(".node[data-node='part:HZ_Kessel.01']");
    assert.equal((await rowNames(page)).length, 4);
    await page.waitForFunction(() => document.querySelector(".detail .facts")?.textContent.includes("Firmware"));
    // an object type
    await page.click("button[data-act=tree-view][data-view=type]");
    assert.equal(await page.$(".node[data-node^='part:']"), null, "by type, no plant parts in between");
    await page.click(".node[data-node='type:4']");
    const types = await page.$$eval(".row[data-point] .type", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(types.length && types.every((type) => type === "BO"), types.join());
    // the device again: all its points in groups, and back to the site
    await page.click(".node[data-act=tree-device]:has-text('DDC4000')");
    await page.waitForSelector(".row.group");
    await showSite(page);
    assert.equal(await page.$$eval(".row[data-device]", (rows) => rows.length), 2);
  },
};
