import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

const names = (page) => page.$$eval(".row[data-point] .name", (cells) => cells.map((cell) => cell.firstChild.textContent.trim()));

// Column heads sort the lists; type chips, views and the search narrow the points, each removable.
export default {
  name: "Sorting and filter by object type",
  async run({ page, base }) {
    await open(page, base);
    await searchInUi(page);
    const objects = () => page.$$eval(".row[data-device] .objects", (cells) => cells.map((cell) => Number(cell.textContent.replace(/\D/g, ""))));
    await page.click("button.sort[data-list=devices][data-key=objects]");
    assert.deepEqual(await objects(), [...await objects()].sort((a, b) => a - b));
    await page.click("button.sort[data-list=devices][data-key=objects]");
    assert.deepEqual(await objects(), [...await objects()].sort((a, b) => b - a));
    await openDevice(page, "AHU-01");
    await page.click("button[data-act=types-menu]");
    await page.click("input[data-act=point-type][data-type='0']");
    const inputs = await page.$$eval(".row[data-point] .type", (cells) => cells.map((cell) => cell.textContent));
    assert.ok(inputs.length > 5 && inputs.every((type) => type === "AI"), `types: ${inputs}`);
    await page.click("button.sort[data-list=points][data-key=name]");
    await page.click("button.sort[data-list=points][data-key=name]");
    const descending = await names(page);
    assert.equal(descending[0], "Supply air temp");
    assert.equal(await page.getAttribute("span[aria-sort=descending] button", "data-key"), "name");
    // several types at once; each chosen one stays as a chip above the list
    await page.click("button[data-act=types-menu]");
    await page.click("input[data-act=point-type][data-type='1']");
    assert.match(await page.textContent("button[data-act=types-menu]"), /AI, AO/);
    await page.keyboard.press("Escape");
    const both = await page.$$eval(".row[data-point] .type", (cells) => [...new Set(cells.map((cell) => cell.textContent))]);
    assert.deepEqual(both.sort(), ["AI", "AO"]);
    assert.equal(await page.$$eval(".filters button[data-act=filter-remove]", (chips) => chips.length), 2);
    // the search knows prefixes: an instance, a type, a value
    await page.fill("#filter", "value:>1000");
    await page.waitForFunction(() => document.querySelector(".filters")?.textContent.includes("value:>1000"));
    await page.click("button[data-act=filters-clear]");
    assert.equal(await page.$(".filters"), null);
    assert.equal(await page.inputValue("#filter"), "");
    await page.fill("#filter", "type:ai");
    const typed = await page.$$eval(".row[data-point] .type", (cells) => [...new Set(cells.map((cell) => cell.textContent))]);
    assert.deepEqual(typed, ["AI"]);
    await page.click(".filters button[data-kind=search]");
    assert.equal(await page.$$eval(".row[data-point] .type", (cells) => new Set(cells.map((cell) => cell.textContent)).size > 2), true);
    assert.equal(await page.$(".row.group"), null, "sorted, the list has no groups");
    await page.click("button.sort[data-list=points][data-key=name]");
    assert.ok(await page.$(".row.group"), "back to the device's order, the groups return");
  },
};
