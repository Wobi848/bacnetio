import { assert, open, searchInUi } from "./helpers.mjs";

// The capture for Wireshark (docs/concept-toolbox.md part 3): started in the settings, it counts
// the packets of a search, shows in the status bar, saves a pcapng file and is cleared.
const packets = async (page) => Number((await page.textContent(".card.capture .figure")).replace(/\D/g, ""));

export default {
  name: "Capture for Wireshark",
  async run({ page, base }) {
    await open(page, base);
    await page.goto(`${base}/#/settings`);
    await page.waitForSelector(".card.capture");
    assert.equal(await packets(page), 0);
    assert.equal(await page.locator(".card.capture a[download]").count(), 0, "nothing to save yet");
    await page.click("[data-act=capture-start]");
    await page.waitForSelector("#status [data-area-go=settings]");
    await page.goto(`${base}/#/workspace`);
    await searchInUi(page);
    await page.click("#status [data-area-go=settings]");
    await page.waitForSelector(".card.capture");
    await page.waitForFunction(() => Number(document.querySelector(".card.capture .figure").textContent.replace(/\D/g, "")) > 0);
    await page.click("[data-act=capture-stop]");
    await page.waitForSelector("[data-act=capture-start]");
    assert.equal(await page.locator("#status [data-area-go=settings]").count(), 0, "no chip once stopped");
    const [file] = await Promise.all([page.waitForEvent("download"), page.click(".card.capture a[download]")]);
    assert.equal(file.suggestedFilename(), "bacnetio.pcapng");
    const chunks = [];
    for await (const chunk of await file.createReadStream()) chunks.push(chunk);
    const bytes = Buffer.concat(chunks);
    assert.equal(bytes.readUInt32LE(0), 0x0a0d0d0a, "a pcapng section header");
    await page.click("[data-act=capture-clear]");
    await page.waitForFunction(() => document.querySelector(".card.capture .figure").textContent.trim().startsWith("0"));
  },
};
