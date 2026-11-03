import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Panels to resize: the details stand beside the list from 1200 px; a dragged border is
// remembered after a reload, a double-click hands the width back, the arrow keys move it too.

const width = (page, selector) => page.evaluate((query) => Math.round(document.querySelector(query).getBoundingClientRect().width), selector);

async function drag(page, panel, by) {
  const box = await page.locator(`.resize[data-panel=${panel}]`).boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + 200);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + by, box.y + 200, { steps: 5 });
  await page.mouse.up();
}

export default {
  name: "Panels to resize, details beside the list",
  async run({ page, base }) {
    await page.setViewportSize({ width: 1250, height: 800 });
    await open(page, base, { area: "workspace" });
    await searchInUi(page);
    await openDevice(page, "DDC4000_#21");
    await page.click(".row[data-point]:has-text('Zuluft_Temperatur')");
    await page.waitForSelector(".detail .big");
    // a column, not a drawer over the list: no way back needed
    assert.equal(await page.locator(".detail .back").isVisible(), false);
    const list = await page.locator(".listwrap").boundingBox();
    const detail = await page.locator(".detail").boundingBox();
    assert.ok(detail.x >= list.x + list.width - 1, "details beside the list");

    const tree = await width(page, ".tree");
    await drag(page, "tree", 60);
    assert.ok(Math.abs(await width(page, ".tree") - (tree + 60)) <= 2, "tree wider by the drag");
    const details = await width(page, ".detail");
    await drag(page, "detail", -40);
    assert.ok(Math.abs(await width(page, ".detail") - (details + 40)) <= 2, "details wider to the left");
    // the list keeps its room however far the border is dragged
    await drag(page, "detail", -800);
    assert.ok(await width(page, ".listwrap") >= 359, "list keeps its room");

    await page.reload();
    await page.waitForSelector(".tree");
    assert.ok(Math.abs(await width(page, ".tree") - (tree + 60)) <= 2, "width remembered");

    await page.dblclick(".resize[data-panel=tree]");
    assert.ok(Math.abs(await width(page, ".tree") - tree) <= 2, "double-click: automatic width");

    await page.focus(".resize[data-panel=side]");
    const side = await width(page, ".side");
    await page.keyboard.press("ArrowRight");
    assert.equal(await width(page, ".side"), side + 16);

    // below 1200 px the details slide over the list again, and their border is gone
    await page.setViewportSize({ width: 1100, height: 800 });
    assert.equal(await page.locator(".resize[data-panel=detail]").isVisible(), false);

    await page.evaluate(() => localStorage.removeItem("bacnetio.panels"));
    await page.setViewportSize({ width: 1500, height: 900 });

    // heads and rows of the point list share their columns, whatever a row holds
    const tracks = (selector) => page.$eval(selector, (row) => getComputedStyle(row).gridTemplateColumns);
    await page.waitForSelector(".row.points:not(.head)");
    assert.equal(await tracks(".row.points.head"), await tracks(".row.points:not(.head)"));
    // a column border dragged: that column wider, in the head and the rows, remembered, and back
    // the value column, wherever it is: a narrow list hides the descriptions
    const value = () => page.$eval(".row.points.head", (head) => [...head.children].filter((cell) => cell.offsetParent).indexOf(head.children[4]));
    const column = async () => Number.parseFloat((await tracks(".row.points:not(.head)")).split(" ")[await value()]);
    const before = await column();
    const border = await page.locator(".row.points.head [role=columnheader]:nth-child(5) .col-border").boundingBox();
    await page.mouse.move(border.x + border.width / 2, border.y + border.height / 2);
    await page.mouse.down();
    await page.mouse.move(border.x + border.width / 2 + 50, border.y + border.height / 2, { steps: 5 });
    await page.mouse.up();
    assert.ok(Math.abs(await column() - (before + 50)) <= 2, "value column wider by the drag");
    assert.equal(await tracks(".row.points.head"), await tracks(".row.points:not(.head)"));
    await page.reload();
    await page.waitForSelector(".row.points:not(.head)");
    // the list settles after the reload, as its points come in
    for (let tries = 0; tries < 20 && Math.abs(await column() - (before + 50)) > 2; tries += 1) await page.waitForTimeout(250);
    assert.ok(Math.abs(await column() - (before + 50)) <= 2, "column width remembered");
    await page.dblclick(".row.points.head [role=columnheader]:nth-child(5) .col-border");
    assert.equal(await page.evaluate(() => localStorage.getItem("bacnetio.columns")), "{}", "double-click: automatic widths");
    assert.equal(await page.textContent("#column-widths"), "");
    await page.evaluate(() => localStorage.removeItem("bacnetio.columns"));
  },
};
