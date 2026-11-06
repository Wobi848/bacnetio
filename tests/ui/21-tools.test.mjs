import { assert, open, openDevice, searchInUi, showSite } from "./helpers.mjs";

// Properties with their types, changing one; the priority that wins per point; sites kept and
// brought back; a device behind a router asks for what it needs.
export default {
  name: "Properties, winning priorities, sites, device behind a router",
  async run({ page, base, api, restart }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    // the winning priority shows on the point once asked for
    await api("devices/1001/objects/1/1/write", "POST", { value: "44", priority: 9 });
    await page.click("button[data-act=more-menu]");
    await page.click("button[data-act=read-winners]");
    await page.waitForSelector(".row[data-point]:has-text('Heating valve') .chip:has-text('Priority 9')", { timeout: 20000 });
    await api("devices/1001/objects/1/1/release", "POST", { priority: 9 });
    // a new device ID: out of range and taken IDs are refused, the device has its say, it is logged
    const newId = (data) => page.request.post(`${base}/api/devices/1001/instance`, { data });
    assert.equal((await newId({ instance: 5000000 })).status(), 400);
    assert.match(await (await newId({ instance: 21 })).text(), /already has the ID 21/);
    await page.click(".node[data-act=tree-device]:has-text('AHU-01')");
    await page.click("button[data-act=toggle-commands]");
    await page.click("button[data-act=ask-instance]");
    await page.click("button[data-act=unlock-instance]");
    await page.fill("#new-instance", "abc");
    await page.click("button[data-act=change-instance]");
    await page.waitForFunction(() => /whole number/.test(document.getElementById("toast").textContent));
    // the simulator does not allow it: the reason shows, the device stays, the log has the attempt
    await page.fill("#new-instance", "1234");
    await page.click("button[data-act=change-instance]");
    await page.waitForFunction(() => /write-access-denied|refused/.test(document.getElementById("toast").textContent));
    assert.ok((await api("log")).some((item) => /object-identifier = Device 1234/.test(item.value ?? "") && item.error));
    await page.click("button[data-act=cancel-write]");
    // properties with a data type; changing one takes the unlock and lands in the log
    await page.click(".row[data-point]:has-text('Outdoor air temp')");
    await page.waitForSelector(".detail .big");
    await page.click(".detail button[data-act=technical]");
    await page.waitForSelector(".detail .props");
    // the BACnet view names each property and its data type as the standard does, and stays chosen
    await page.click("button[data-act=property-names][data-names=bacnet]");
    await page.waitForFunction(() => /present-value · real/.test(document.querySelector(".detail").textContent));
    await page.click("button[data-act=property-names][data-names=plain]");
    await page.click("button[data-act=property-edit][data-property=out-of-service]");
    await page.fill("#property-value", "true");
    await page.click("button[data-act=property-write]");
    await page.waitForFunction(() => /out-of-service = true/.test(document.getElementById("toast").textContent));
    // out of service counts as held until it is taken back, here on the page as well
    await page.click("button[data-act=property-edit][data-property=out-of-service]");
    await page.fill("#property-value", "false");
    await page.click("button[data-act=property-write]");
    await page.waitForFunction(() => !document.querySelector(".status .chips .chip"));
    const log = await api("log");
    const entry = log.find((item) => /out-of-service/.test(item.value ?? ""));
    assert.match(entry.value, /out-of-service/);
    assert.equal(entry.error, null);
    await page.keyboard.press("5");
    await page.waitForFunction(() => /wrote out-of-service/.test(document.querySelector(".rows").textContent));
    // the search is kept as a site, shown again after a restart
    await restart();
    await page.goto(`${base}/#/sites`);
    await page.reload();
    await page.waitForSelector(".sites .site.open");
    assert.match(await page.textContent(".sites .site.open"), /127\.0\.0\.0\/29/);
    // the site gets a name of the technician's, which the sidebar shows: asked for in the sidebar
    // while it is known only by its network, the name field opens ready to type
    await page.goto(`${base}/#/workspace`);
    await page.click(".site-group button[data-act=site-name]");
    await page.waitForFunction(() => location.hash === "#/sites" && document.activeElement?.id === "site-title");
    await page.keyboard.type("Sample School");
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => document.querySelector(".site-group .site-now")?.textContent === "Sample School");
    assert.equal((await api("sites"))[0].title, "Sample School");
    // a site file saves and loads again; what is no site file is refused
    const site = await api("site.json");
    assert.ok(site.scan.devices.length === 2);
    const refused = await page.request.post(`${base}/api/site.json`, { data: { name: "x" } });
    assert.equal(refused.status(), 400);
    const script = await page.request.post(`${base}/api/site.json`, { data: { name: "x", scan: { state: "done", devices: [{ instance: "<img src=x onerror=alert(1)>", network: null }] } } });
    assert.equal(script.status(), 400);
    // a file that would wedge the page: a search that never ends, a value the page reads as text
    const running = await page.request.post(`${base}/api/site.json`, { data: { ...site, scan: { ...site.scan, state: "running" } } });
    assert.equal(running.status(), 400);
    const wrongValue = await page.request.post(`${base}/api/site.json`, { data: { name: "x", scan: { state: "done", devices: [] }, points: { 21: { points: [{ type: 0, instance: 1, value: 21.5 }] } } } });
    assert.equal(wrongValue.status(), 400);
    const imported = await page.request.post(`${base}/api/site.json`, { data: { ...site, name: "Plant B", title: undefined } });
    assert.equal(imported.status(), 204);
    assert.ok((await api("sites")).some((kept) => kept.name === "Plant B"));
    // each kept site saves to a file of its own, not only the one shown
    const other = await page.request.get(`${base}/api/site.json?name=127.0.0.0%2F29`);
    assert.equal((await other.json()).title, "Sample School");
    // a new search of the network named before brings its name back
    await page.goto(`${base}/#/workspace`);
    await page.reload();
    await page.waitForFunction(() => document.querySelector(".site-group .site-now")?.textContent === "Plant B");
    await searchInUi(page);
    await page.waitForFunction(() => document.querySelector(".site-group .site-now")?.textContent === "Sample School");
    // another kept site opens from the list
    await page.click(".site-link");
    await page.click(".sites li:has-text('Plant B') button[data-act=site-open]");
    await page.waitForFunction(() => document.querySelector(".site-group .site-now")?.textContent === "Plant B");
    // its devices follow the name, a moment later in some browsers
    await page.waitForFunction(() => document.querySelectorAll(".node[data-act=tree-device]").length === 2, null, { timeout: 5000 });
    // a kept site can be removed, after a question
    await page.click(".site-link");
    await page.click(".sites li:has-text('Plant B') button[data-act=site-ask-delete]");
    await page.click("button[data-act=site-delete]");
    await page.waitForFunction(() => !/Plant B/.test(document.querySelector(".sites")?.textContent ?? ""));
    assert.ok(!(await api("sites")).some((kept) => kept.name === "Plant B"));
    // a file that is no JSON says so in a sentence, not with the engine's parser message
    await page.setInputFiles("#site-file", { name: "broken.json", mimeType: "application/json", buffer: Buffer.from("<html>no site file</html>") });
    await page.waitForFunction(() => /no bacnetio site file/.test(document.getElementById("toast")?.textContent ?? ""));
    assert.doesNotMatch(await page.textContent("#toast"), /token|position/i);
    // a device behind a router needs its ID and a valid station address
    await page.goto(`${base}/#/workspace`);
    await page.reload();
    const locate = (data) => page.request.post(`${base}/api/devices/locate`, { data: { interface: "127.0.0.1", port: 47809, address: "127.0.0.2:47809", network: 2, ...data } });
    assert.equal((await locate({ mac: "12" })).status(), 400);
    assert.equal((await locate({ mac: "nonsense", instance: 5 })).status(), 400);
    await showSite(page);
  },
};
