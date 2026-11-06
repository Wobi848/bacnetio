import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SIM_PORT, assert, open, openDevice, searchInUi } from "./helpers.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

// The plant view a device carries itself (Structured View objects) stands in the tree, a branch
// shows its points, nothing is lost, and a device without such objects keeps the grouping by names.
export default {
  name: "Plant view from Structured View objects",
  async run({ page, base }) {
    // beside the plant a controller with neither a plant view nor path names
    const exe = process.platform === "win32" ? ".exe" : "";
    const flat = spawn(path.join(root, `target/debug/examples/simulated_commands${exe}`), ["127.0.0.4", String(SIM_PORT)], { stdio: "ignore" });
    try {
      await this.check(page, base);
    } finally {
      flat.kill();
    }
  },

  async check(page, base) {
    await open(page, base);
    await searchInUi(page, 3);
    await openDevice(page, "AHU-01");
    await page.click("button[data-act=tree-view][data-view=plant]");
    await page.waitForSelector(".node[data-node='view:1']", { timeout: 20000 });
    const nodes = await page.$$eval(".tree .node[data-node^='view:'] .label", (cells) => cells.map((cell) => cell.textContent));
    assert.deepEqual(nodes, ["AHU-01 plant", "Air handling", "Rooms", "Not in the plant view"]);
    assert.match(await page.textContent(".tree .node.heading:has-text('Plant')"), /Plant/);
    // the root counts everything below it, through its two views
    assert.equal(await page.textContent(".node[data-node='view:1'] .count"), "17");
    assert.equal(await page.textContent(".node[data-node='view:2'] .count"), "7");
    assert.equal(await page.textContent(".node[data-node='view:3'] .count"), "10");
    // a branch shows its own points, in the order the view names them
    await page.click(".node[data-node='view:2']");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point]").length === 7);
    const names = await page.$$eval(".row[data-point] .name", (cells) => cells.map((cell) => cell.textContent.trim()));
    assert.ok(names[0].startsWith("Supply air temp"), names.join(", "));
    assert.ok(names.some((name) => name.startsWith("Operating mode")), names.join(", "));
    assert.ok(!names.some((name) => name.startsWith("Room temp")), names.join(", "));
    await page.click(".node[data-node='view:3']");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point]").length === 10);
    // what no view holds stays reachable: the schedule and the trend log
    await page.click(".node[data-node='view:outside']");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point]").length === 2);
    const rest = await page.textContent(".rows");
    assert.match(rest, /Office hours/);
    assert.match(rest, /Supply air temp log/);
    // the whole device again: the views themselves are objects of it too
    await page.click(".node[data-act=tree-device]:has-text('AHU-01')");
    await page.waitForFunction(() => document.querySelectorAll(".row[data-point]").length === 22);
    // the controller has no Structured View objects, so its names still give the groups
    await openDevice(page, "DDC4000_#21");
    await page.waitForSelector(".tree .node[data-node^='part:']", { timeout: 20000 });
    assert.equal(await page.$$eval(".tree .node[data-node^='view:']", (rows) => rows.length), 0);
    const parts = await page.$$eval(".tree .node[data-node^='part:'] .label", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(parts.includes("SY_Datsi.01"), parts.join(", "));
    // a device read but without a plant view or path names says so in the plant tree, whole
    await openDevice(page, "Commands-01");
    const note = page.locator(".tree .tree-note");
    assert.match(await note.textContent(), /^No plant parts: .* All its objects are under By type\.$/);
    assert.ok(await note.evaluate((element) => element.scrollWidth <= element.clientWidth), "not cut off");
  },
};
