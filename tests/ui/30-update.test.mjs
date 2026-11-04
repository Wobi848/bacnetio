import { assert, open } from "./helpers.mjs";

// A newer release: a note in the sidebar and in the settings. Asked once a day, again on request;
// switched off, the page never asks.
const RELEASE = "https://github.com/Wobi848/bacnetio/releases/tag/v9.0.0";

export default {
  name: "New version: note, once a day, switch",
  async run({ page, base }) {
    let asked = 0;
    let answer = { json: { current: "0.1.0", latest: { version: "9.0.0", url: RELEASE }, newer: true } };
    await page.route("**/api/update", (route) => {
      asked += 1;
      return route.fulfill(answer);
    });
    await open(page, base, { area: "settings" });
    await page.waitForSelector(".update-note");
    assert.equal(await page.textContent(".update-note"), "New version 9.0.0");
    assert.equal(await page.getAttribute(".update-note", "href"), RELEASE);
    assert.match(await page.textContent(".update-status"), /Version 9\.0\.0 is out/);

    // within a day the kept answer stands, without asking
    await page.reload();
    await page.waitForSelector(".update-note");
    assert.equal(asked, 1);

    // on request it asks again; GitHub unreachable says so, and the note goes
    answer = { status: 502, json: { error: "no connection to GitHub" } };
    await page.click("button[data-act=update-now]");
    await page.waitForFunction(() => /No answer from GitHub/.test(document.querySelector(".update-status").textContent));
    assert.equal(await page.locator(".update-note").count(), 0);

    // switched off: no asking, also after a reload
    await page.click("button[data-act=update-check][data-on=false]");
    assert.match(await page.textContent(".update-status"), /does not go online/);
    const before = asked;
    await page.reload();
    await page.waitForSelector("#adapter", { state: "attached" });
    await page.waitForTimeout(300);
    assert.equal(asked, before);

    // on again: asks at once
    answer = { json: { current: "0.1.0", latest: { version: "0.1.0", url: RELEASE }, newer: false } };
    await page.click("button[data-act=update-check][data-on=true]");
    await page.waitForFunction(() => /newest version/.test(document.querySelector(".update-status").textContent));
    assert.equal(asked, before + 1);
  },
};
