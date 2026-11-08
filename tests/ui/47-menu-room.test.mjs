import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// A toolbar menu in a low or narrow window: the work area cut off its lower entries, and its left
// side when the button had wrapped to a line of its own. Every entry can be reached and clicked.
async function reachable(page, selector) {
  return page.$eval(selector, (element) => {
    element.scrollIntoView({ block: "nearest" });
    const box = element.getBoundingClientRect();
    const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    return element.contains(hit) && box.bottom <= innerHeight && box.left >= 0;
  });
}

export default {
  name: "Toolbar menus stay inside a low or narrow window",
  async run({ page, base }) {
    await page.setViewportSize({ width: 1280, height: 600 });
    await open(page, base, { area: "workspace" });
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    for (const size of [{ width: 1280, height: 600 }, { width: 900, height: 600 }]) {
      await page.setViewportSize(size);
      await page.click("button[data-act=more-menu]");
      const items = ".menu.exports > a:not(.drawer-only), .menu.exports > button:not(.drawer-only)";
      await page.waitForSelector(items);
      assert.ok(await reachable(page, `${items} >> nth=0`), `the first entry at ${size.width}×${size.height}`);
      assert.ok(await reachable(page, "button[data-act=read-winners]"), `the last entry at ${size.width}×${size.height}`);
      await page.keyboard.press("Escape");
      await page.waitForFunction(() => !document.querySelector(".menu.exports"));
    }
  },
};
