import { assert, open } from "./helpers.mjs";

// The help opens with ? and from the sidebar, shows a guide with the interface's button names and
// leads to the place of its first step; the keys moved there from the settings (concept-help.md).
export default {
  name: "Help: guides, the key ?, the way to the first step",
  async run({ page, base }) {
    await open(page, base, { language: "de" });
    await page.click(".pagehead");
    await page.keyboard.press("?");
    await page.waitForSelector(".help-guide");
    assert.equal(await page.evaluate(() => location.hash), "#/help");
    assert.equal(await page.textContent(".pagehead h1"), "Hilfe");
    assert.equal(await page.getAttribute("a.help-link", "aria-current"), "page");

    // a guide names the buttons as they are written in the interface
    await page.click("button[data-guide=vpn]");
    assert.equal(await page.textContent("#help-guide-title"), "Anlage hinter einem VPN erreichen");
    assert.equal(await page.getAttribute("button[data-guide=vpn]", "aria-pressed"), "true");
    const named = await page.$$eval(".help-steps b", (bold) => bold.map((one) => one.textContent));
    assert.ok(named.includes("Sucheinstellungen") && named.includes("Mit diesen Einstellungen suchen"), named.join(", "));

    // its button opens the search settings, and only opens them
    await page.click("button[data-act=help-go]");
    await page.waitForSelector(".modal-box.search-settings");
    assert.equal(await page.evaluate(() => location.hash), "#/workspace");
    await page.keyboard.press("Escape");

    // the keys: from the settings to the help
    await page.click("a.settings-link:not(.help-link)");
    await page.click("button[data-act=help-guide][data-guide=keys]");
    await page.waitForSelector("#help-guide kbd");
    assert.equal(await page.textContent("#help-guide-title"), "Tasten");
    assert.equal(await page.evaluate(() => location.hash), "#/help");

    // every guide opens, and each button leads to its place
    const guides = await page.$$eval("button[data-act=help-guide]", (buttons) => buttons.map((button) => button.dataset.guide));
    assert.equal(guides.length, 15);
    for (const [guide, hash] of [["watch", "#/watch"], ["record", "#/recording"], ["capture", "#/settings"], ["sites", "#/sites"]]) {
      await page.goto(`${base}/#/help`);
      await page.click(`button[data-guide=${guide}]`);
      await page.waitForSelector(".help-steps li");
      await page.click("button[data-act=help-go]");
      await page.waitForFunction((want) => location.hash === want, hash);
    }
    await page.goto(`${base}/#/help`);

    // the guide that writes leads to the values held, without writing anything
    await page.click("button[data-guide=write]");
    await page.click("button[data-act=help-go]");
    assert.equal(await page.evaluate(() => location.hash), "#/log");
  },
};
