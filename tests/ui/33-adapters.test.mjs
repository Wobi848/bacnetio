import { assert, open } from "./helpers.mjs";

// The adapter list is read again while the page is open: a new address shows without a restart,
// and the chosen adapter stays chosen.
export default {
  name: "Adapters are read again: a new address shows",
  async run({ page, base }) {
    await open(page, base);
    await page.waitForSelector("#adapter", { state: "attached" });
    const label = () => page.$eval("#adapter", (select) => select.selectedOptions[0].textContent);
    const first = await label();
    await page.route("**/api/adapters", async (route) => {
      const adapters = await (await route.fetch()).json();
      route.fulfill({ json: adapters.map((adapter) => ({ ...adapter, ip: "127.0.0.9" })) });
    });
    // coming back to the window asks at once
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await page.waitForFunction(() => document.querySelector("#adapter option:checked")?.textContent.includes("127.0.0.9"));
    assert.equal((await label()).split(" · ")[0], first.split(" · ")[0]);
  },
};
