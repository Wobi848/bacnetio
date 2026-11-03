import { assert, open, openDevice, searchInUi } from "./helpers.mjs";

// Commands to a device ask first and say what follows; the password is sent, never logged. Any plain
// property can be changed. The network can be asked who has a name.
export default {
  name: "Device commands, any property, Who-Has",
  async run({ page, base, api }) {
    await open(page, base);
    await searchInUi(page);
    await openDevice(page, "AHU-01");
    await page.click("button[data-act=toggle-commands]");
    await page.click("button[data-act=ask-command][data-command=warm]");
    assert.match(await page.textContent(".ask"), /restarts/);
    await page.click("button[data-act=unlock-command]");
    await page.fill("#cmd-password", "secret-word");
    await page.click("button[data-act=run-command]");
    await page.waitForFunction(() => !document.getElementById("toast").hidden);
    const log = await api("log");
    const entry = log.find((item) => item.action === "command");
    assert.match(entry.value, /warm start/);
    assert.doesNotMatch(JSON.stringify(log), /secret-word/, "the password stays out of the log");
    // a command must not break the page that lists every change
    await page.keyboard.press("5");
    await page.waitForFunction(() => /warm start/.test(document.body.textContent));
    await page.keyboard.press("1");
    // the simulator may refuse a restart; the question then stays open
    if (await page.$("button[data-act=cancel-write]")) await page.click("button[data-act=cancel-write]");
    // a stop never lasts zero minutes, and a slip in the field stops nothing
    const zero = await page.request.post(`${base}/api/devices/1001/communication`, { data: { enable: false, minutes: 0 } });
    assert.equal(zero.status(), 400);
    await page.click("button[data-act=ask-command][data-command=stop]");
    await page.fill("#cmd-minutes", "abc");
    await page.click("button[data-act=run-command]");
    await page.waitForFunction(() => /whole number of minutes/.test(document.getElementById("toast").textContent));
    assert.equal((await api("log")).filter((item) => /communication off/.test(item.value ?? "")).length, 0);
    await page.click("button[data-act=cancel-write]");
    // a new name for the device, the old one to start from; the simulator refuses it in words, and
    // the name stays everywhere (a device that takes it is checked on a real plant)
    await page.click("button[data-act=ask-name]");
    assert.equal(await page.inputValue("#new-name"), "AHU-01");
    await page.fill("#new-name", "AHU-01 North");
    await page.click("button[data-act=change-name]");
    await page.waitForFunction(() => /does not allow this to be changed/.test(document.getElementById("toast").textContent));
    assert.match(await page.textContent(".detail .dhead"), /AHU-01/);
    assert.doesNotMatch(await page.textContent(".detail .dhead"), /North/);
    assert.ok((await api("scan")).devices.some((device) => device.name === "AHU-01"));
    await page.click("button[data-act=cancel-write]");
    assert.equal((await page.request.post(`${base}/api/who-has`, { data: { name: "  " } })).status(), 400);
    const notFinite = await page.request.post(`${base}/api/devices/1001/objects/0/1/property`, { data: { property_id: 22, data_type: "real", value: "NaN" } });
    assert.match(await notFinite.text(), /not a number/);
    // an object out of service counts as held until it is taken back
    await api("devices/1001/objects/0/2/property", "POST", { property_id: 81, data_type: "boolean", value: "true" });
    assert.ok((await api("held")).some((entry) => entry.type === 0 && entry.instance === 2));
    await api("devices/1001/objects/0/2/property", "POST", { property_id: 81, data_type: "boolean", value: "false" });
    assert.equal((await api("held")).length, 0);
    // a bad kind is refused
    const bad = await page.request.post(`${base}/api/devices/1001/restart`, { data: { kind: "hot" } });
    assert.equal(bad.status(), 400);
    // what cannot be written here says so
    const reserved = await page.request.post(`${base}/api/devices/1001/objects/0/1/property`, { data: { property_id: 85, data_type: "real", value: "1" } });
    assert.match(await reserved.text(), /present-value cannot be changed here/);
    const wrong = await page.request.post(`${base}/api/devices/1001/objects/0/1/property`, { data: { property_id: 22, data_type: "real", value: "abc" } });
    assert.match(await wrong.text(), /not a number/);
    // a schedule reads in words
    await page.click(".row[data-point]:has-text('Office hours')");
    await page.waitForSelector(".detail .times");
    assert.match(await page.textContent(".detail .times"), /06:00[\s\S]*21\.5[\s\S]*18:00[\s\S]*17/);
    // a day of the week can be changed, after the unlock, and it lands in the log
    await page.click("button[data-act=day-edit][data-day='5']");
    await page.fill("#day-changes", "08:00 19.5; 12:00 18");
    await page.click("button[data-act=day-write]");
    await page.waitForFunction(() => /08:00[\s\S]*19\.5/.test(document.querySelector(".detail .times")?.textContent ?? ""));
    assert.ok((await api("log")).some((item) => /day 6 of the week: 08:00 19.5/.test(item.value ?? "") && !item.error));
    await page.click("button[data-act=day-edit][data-day='5']");
    await page.fill("#day-changes", "no time");
    await page.click("button[data-act=day-write]");
    await page.waitForFunction(() => !document.getElementById("toast").hidden && /needs a time|no time/.test(document.getElementById("toast").textContent));
    await page.click("button[data-act=day-cancel]");
    // the network can be asked who has a name
    await page.fill("#site-search", "Supply air temp");
    await page.click("button[data-act=who-has]");
    await page.waitForFunction(() => !document.querySelector("button[data-act=who-has]")?.disabled, null, { timeout: 15000 });
    assert.match(await page.textContent(".rows"), /Supply air temp/);
  },
};
