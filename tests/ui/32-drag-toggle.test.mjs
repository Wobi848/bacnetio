import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Press on a switch in the point list and drag over other rows: they all take the new state of the
// first. A plain click still switches one.
const ROWS = ["Room temp 2.01", "Room temp 2.02", "Room temp 2.03"];
const eye = (name) => `.row[data-point]:has-text('${name}') button[data-act=quick-watch]`;
const watched = (page) => page.$$eval("button[data-act=quick-watch][aria-pressed=true]", (buttons) => buttons.length);

async function centre(page, selector) {
  const box = await page.locator(selector).boundingBox();
  return [box.x + box.width / 2, box.y + box.height / 2];
}

export default {
  name: "Drag over the switches to set many at once",
  async run({ page, base }) {
    await open(page, base, { area: "workspace" });
    await searchInUi(page);
    await openDevice(page, "AHU-01");

    const [x, y] = await centre(page, eye(ROWS[0]));
    const [, lastY] = await centre(page, eye(ROWS[2]));
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x, (y + lastY) / 2, { steps: 4 });
    await page.mouse.move(x, lastY, { steps: 4 });
    await page.mouse.up();
    await page.waitForFunction(() => document.querySelectorAll("button[data-act=quick-watch][aria-pressed=true]").length === 3);
    for (const name of ROWS) assert.equal(await page.getAttribute(eye(name), "aria-pressed"), "true", name);
    assert.equal(await page.$$eval(".detail.open", (open) => open.length), 0, "the drag opened no point");

    // starting on a switch that is on switches them off again
    await page.mouse.move(x, lastY);
    await page.mouse.down();
    await page.mouse.move(x, y, { steps: 8 });
    await page.mouse.up();
    await page.waitForFunction(() => !document.querySelector("button[data-act=quick-watch][aria-pressed=true]"));

    // record goes through the server, one request after the other
    const chart = (name) => `.row[data-point]:has-text('${name}') button[data-act=quick-record]`;
    const [rx, ry] = await centre(page, chart(ROWS[0]));
    await page.mouse.move(rx, ry);
    await page.mouse.down();
    await page.mouse.move(rx, lastY, { steps: 8 });
    await page.mouse.up();
    await page.waitForFunction(() => document.querySelectorAll("button[data-act=quick-record][aria-pressed=true]").length === 3);
    await page.mouse.move(rx, lastY);
    await page.mouse.down();
    await page.mouse.move(rx, ry, { steps: 8 });
    await page.mouse.up();
    await page.waitForFunction(() => !document.querySelector("button[data-act=quick-record][aria-pressed=true]"));

    // a plain click switches one, once
    await page.click(eye(ROWS[1]));
    await page.waitForTimeout(300);
    assert.equal(await watched(page), 1);
    await page.click(eye(ROWS[1]));
    await page.waitForTimeout(300);
    assert.equal(await watched(page), 0);
  },
};
