import fs from "node:fs";
import path from "node:path";
import { assert, open } from "./helpers.mjs";

// BACnet/SC, step 3 (docs/concept-sc.md): a key without a password is kept for the next start,
// *Forget the SC key* in the settings deletes it and the certificates stay; the capture says it
// records nothing on SC. Against simulated_sc, which the runner starts.
export default {
  name: "BACnet/SC: the kept key forgotten in the settings, the capture on SC",
  async run({ page, base, api }) {
    const dir = process.env.BACNETIO_TEST_SC_DIR;
    if (!dir) return;
    for (const [line, name] of [["ca", "ca.pem"], ["own", "bacnetio.pem"], ["key", "bacnetio.key"]]) {
      const response = await fetch(`${base}/api/sc/file?line=${line}&name=${name}`, {
        method: "POST",
        headers: { "content-type": "application/octet-stream" },
        body: fs.readFileSync(path.join(dir, name)),
      });
      assert.ok(response.ok, `${name}: ${response.status}`);
    }
    await api("sc/search", "POST", { hub: process.env.BACNETIO_TEST_SC_HUB });
    for (let attempt = 0; attempt < 100 && (await api("scan")).state === "running"; attempt++) {
      await new Promise((done) => setTimeout(done, 100));
    }
    assert.equal((await api("sc")).key_kept, true);

    await open(page, base, { area: "settings" });
    await page.waitForSelector(".card.sc-key");
    assert.match(await page.textContent(".card.capture"), /records only BACnet\/IP and MS\/TP/);
    assert.match(await page.textContent(".card.sc-key"), /kept on this computer, readable only for you/);
    await page.click("[data-act=sc-forget-key]");
    await page.waitForFunction(() => /No SC key is kept/.test(document.querySelector(".card.sc-key").textContent));
    const after = await api("sc");
    assert.equal(after.key_kept, false);
    assert.equal(after.certificates.key.state, "missing");
    assert.equal(after.certificates.ca.state, "valid");
  },
};
