import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// In every language, width and theme: nothing runs out of the window, no button, field or state
// chip is cut off, the page never scrolls sideways. With a controller of 170+ points and long names.

const AREAS = ["workspace", "problems", "watch", "recording", "log", "settings", "sites"];
const WIDTHS = [390, 1000, 1250, 1600];
const LANGUAGES = ["en", "de", "fr", "it", "es"];
const THEMES = ["light", "dark"];

const problems = (page) =>
  page.evaluate(() => {
    const box = (element) => element.getBoundingClientRect();
    const shown = (element) => box(element).width > 0 && box(element).height > 0 && getComputedStyle(element).visibility !== "hidden";
    const out = [];
    for (const element of document.querySelectorAll("button, a.quiet, a.button, select, input")) {
      if (!shown(element) || element.closest(".tabs, .seg, .menu, .detail:not(.open), .rows")) continue;
      if (box(element).right > innerWidth + 1 || box(element).left < -1) out.push(`outside: ${(element.textContent || element.id).trim().slice(0, 30)}`);
    }
    for (const element of document.querySelectorAll(".toolbar button, .toolbar a.quiet, #next, .card button, .detail button, .net button, .net .state, .pagehead h1")) {
      if (shown(element) && element.scrollWidth > element.clientWidth + 1) out.push(`cut off: ${element.textContent.trim().slice(0, 30)}`);
    }
    // a chip is a state: it must be readable whole, not cut off by its row or column
    for (const chip of document.querySelectorAll("#work .chip, .status .chip")) {
      if (!shown(chip)) continue;
      if (chip.scrollWidth > chip.clientWidth + 1) out.push(`chip cut off: ${chip.textContent.trim().slice(0, 30)}`);
      for (let up = chip.parentElement; up && up !== document.body; up = up.parentElement) {
        if (getComputedStyle(up).overflowX !== "visible" && box(chip).right > box(up).right + 1) {
          out.push(`chip cut off: ${chip.textContent.trim().slice(0, 30)}`);
          break;
        }
      }
    }
    // a group head sticks above its rows; it must not lie on the first of them
    for (const group of document.querySelectorAll(".rows .row.group")) {
      const next = group.nextElementSibling;
      if (shown(group) && next && shown(next) && box(next).top < box(group).bottom - 1) out.push(`group covers a row: ${group.textContent.trim().slice(0, 30)}`);
    }
    if (document.documentElement.scrollWidth > innerWidth) {
      const wide = [...document.querySelectorAll("body *")].filter((element) => shown(element) && box(element).right > innerWidth + 1 && !element.closest(".tabs"));
      out.push(`page too wide (${document.documentElement.scrollWidth} px): ${wide.slice(0, 3).map((element) => element.id || element.className || element.tagName).join(", ")}`);
    }
    return out;
  });

export default {
  name: "Layout (languages, widths, themes, areas, 170+ points, long names)",
  async run({ page, base }) {
    await open(page, base);
    const found = [];
    // before the first search: the sidebar says "Ready to search", the device list is empty
    for (const language of LANGUAGES) {
      await page.evaluate((lang) => localStorage.setItem("bacnetio.language", lang), language);
      for (const width of WIDTHS) {
        await page.setViewportSize({ width, height: 800 });
        for (const area of ["workspace", "problems", "watch"]) {
          await page.goto(`${base}/#/${area}`);
          await page.reload();
          await page.waitForSelector("#adapter");
          for (const problem of await problems(page)) found.push(`not searched, ${language}, ${width} px, ${area}: ${problem}`);
        }
      }
    }
    await page.evaluate(() => localStorage.setItem("bacnetio.language", "en"));
    await page.setViewportSize({ width: 1500, height: 900 });
    await page.goto(`${base}/#/workspace`);
    await page.reload();
    await page.waitForSelector("#adapter");
    await searchInUi(page);
    await openDevice(page, "DDC4000_#21");
    await page.click(".row[data-point]:has-text('Zuluft_Temperatur')");
    await page.waitForSelector(".detail .big");
    await page.click(".detail button[data-act=rec-add]");
    await page.click(".detail button[data-act=watch-add]");
    await page.click(".row.group:has-text('RA_Zone.07')");
    for (const language of LANGUAGES) {
      for (const theme of THEMES) {
        await page.evaluate(([lang, look]) => {
          localStorage.setItem("bacnetio.language", lang);
          localStorage.setItem("bacnetio.theme", look);
        }, [language, theme]);
        for (const width of WIDTHS) {
          await page.setViewportSize({ width, height: 800 });
          for (const area of AREAS) {
            // a query of its own makes this one full load; a changed hash alone would not reload
            await page.goto(`${base}/?${language}-${theme}-${width}#/${area}`);
            await page.waitForSelector("#adapter");
            await page.waitForTimeout(area === "points" ? 600 : 200);
            for (const problem of await problems(page)) found.push(`${language}, ${theme}, ${width} px, ${area}: ${problem}`);
          }
        }
      }
    }
    assert.deepEqual([...new Set(found)], []);
  },
};
