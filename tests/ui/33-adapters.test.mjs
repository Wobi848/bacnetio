import { assert, open } from "./helpers.mjs";

// The adapter list is read again while the page is open: a new address shows without a restart,
// and the chosen adapter stays chosen.
export default {
  name: "Adapters are read again: a new address shows",
  async run({ page, base }) {
    await open(page, base);
    await page.waitForSelector(".side-card", { state: "attached" });
    // the sidebar names the adapter and its address; the choice itself is in the search settings
    const label = () => page.$eval(".side-card .net", (part) => `${part.querySelector(".adapter-name").textContent} · ${part.querySelector(".adapter-ip").textContent}`);
    const first = await label();
    await page.route("**/api/adapters", async (route) => {
      const adapters = await (await route.fetch()).json();
      route.fulfill({ json: adapters.map((adapter) => ({ ...adapter, ip: "127.0.0.9" })) });
    });
    // coming back to the window asks at once
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await page.waitForFunction(() => document.querySelector(".side-card .adapter-ip")?.textContent === "127.0.0.9");
    await page.click(".side button[data-act=network]");
    assert.match(await page.$eval("#adapter", (select) => select.selectedOptions[0].textContent), /127\.0\.0\.9/);
    assert.equal((await label()).split(" · ")[0], first.split(" · ")[0]);
  },
};
