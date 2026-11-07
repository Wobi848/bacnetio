import { assert, open } from "./helpers.mjs";

// Language, theme, accent colour and background apply at once and stay after a reload.
export default {
  name: "Settings: language and theme",
  async run({ page, base }) {
    await open(page, base, { area: "settings" });
    // no next step on this page, so nothing on it is filled: a chosen choice is light, every tool quiet
    const filled = await page.$$eval(".work button, .work a, .work label.button", (controls) => controls.filter((control) => {
      const ground = getComputedStyle(control).backgroundColor;
      const accent = getComputedStyle(document.documentElement).getPropertyValue("--accent").trim();
      const probe = document.createElement("span");
      probe.style.color = accent;
      document.body.append(probe);
      const accentColor = getComputedStyle(probe).color;
      probe.remove();
      return ground === accentColor;
    }).map((control) => control.textContent.trim()));
    assert.deepEqual(filled, []);
    // the version large, once bacnetio has said which it is
    await page.waitForFunction(() => /\d+\.\d+\.\d+/.test(document.querySelector(".card .figure")?.textContent ?? ""));
    await page.click("button[data-code=de]");
    assert.equal(await page.textContent(".pagehead h1"), "Einstellungen");
    await page.click("button[data-theme=dark]");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), "dark");
    await page.click("button[data-accent=crimson]");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.accent), "crimson");
    // a background per theme: the dark one applies now, the light one when the theme changes
    await page.click("button[data-act=background][data-background=graphite]");
    await page.click("button[data-act=background][data-background=warm]");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.background), "graphite");
    assert.equal(await page.evaluate(() => getComputedStyle(document.body).backgroundColor), "rgb(19, 20, 22)");
    await page.reload();
    await page.waitForSelector(".side-card", { state: "attached" });
    assert.equal(await page.textContent(".pagehead h1"), "Einstellungen");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), "dark");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.accent), "crimson");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.background), "graphite");
    await page.click("button[data-theme=light]");
    assert.equal(await page.evaluate(() => document.documentElement.dataset.background), "warm");
    // French, Italian and Spanish wait for a review: not in the menu, but open with ?lang=
    const menu = await page.$$eval("button[data-code]", (buttons) => buttons.map((button) => button.dataset.code));
    assert.deepEqual(menu, ["auto", "en", "de"]);
    await page.goto(`${base}/?lang=fr#/settings`);
    await page.waitForSelector(".side-card", { state: "attached" });
    assert.equal(await page.textContent(".pagehead h1"), "Réglages");
  },
};
