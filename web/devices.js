// The device list: the search in place with its progress, the devices found, and the details of
// the chosen one beside the list.

import { bbmdHtml } from "./bbmd.js";
import { deviceCount, deviceMaker, deviceName, errorStrip, noteStrip, ownObjects } from "./format.js";
import { h } from "./html.js";
import { currentLocale, errorText, eventStateText, objectText, t } from "./i18n.js";
import { listBorders } from "./panels.js";
import { searchScope } from "./shell.js";
import { sortHead, sortRows } from "./sorting.js";

const DEVICE_COLUMNS = {
  name: deviceName,
  id: (device) => device.instance,
  address: (device) => device.address,
  vendor: deviceMaker,
  objects: (device) => device.objects?.total ?? null,
};

/** Devices matching every word of the search, in name, vendor, model, ID and address. */
export function filterDevices(devices, filter) {
  const words = filter.toLowerCase().split(/\s+/).filter(Boolean);
  return devices.filter((device) => {
    const text = [device.name, device.vendor, device.model, device.instance, device.address].join(" ").toLowerCase();
    return words.every((word) => text.includes(word));
  });
}

function progress(scan) {
  if (scan.state !== "running") return "";
  const share = scan.phase === "sweep" && scan.total ? Math.round((scan.asked / scan.total) * 100) : scan.phase === "details" || scan.phase === "objects" ? 100 : 10;
  return `<span class="progress"><span class="dot busy"></span>${t("found", { n: scan.found ?? 0 })}<span class="track"><div style="width:${share}%"></div></span></span>`;
}

function locateMenu(state) {
  if (state.menu !== "locate") return "";
  return `<div class="menu dialog end" role="dialog" aria-label="${t("locate.label")}">
    <h3>${t("locate.label")}</h3><p class="hint">${t("locate.hint")}</p>
    <input id="locate-address" class="mono" placeholder="192.168.10.20" inputmode="decimal" value="${h(state.settings.locate)}">
    <p class="hint">${t("locate.routedHint")}</p>
    <label class="row"><span>${t("net")}</span><input id="locate-network" class="mono" inputmode="numeric" placeholder="2" value="${h(state.settings.locateNetwork)}"></label>
    <label class="row"><span>MAC</span><input id="locate-mac" class="mono" placeholder="12" value="${h(state.settings.locateMac)}"></label>
    <label class="row"><span>${t("dev.id")}</span><input id="locate-id" class="mono" inputmode="numeric" value="${h(state.settings.locateId)}"></label>
    <div class="row end"><button type="button" class="quiet" data-act="close-menu">${t("cancel")}</button><button type="button" data-act="locate">${t("locate.open")}</button></div></div>`;
}

function exportMenu(state, searching) {
  const menu = state.menu !== "export" ? "" : `<div class="menu end exports" role="menu">
    <a class="quiet" href="/api/devices.csv" download>${t("export.devices")}</a>
    <button type="button" class="quiet" data-act="site-ede" title="${t("ede.siteHint")}"${searching || state.reading ? " disabled" : ""}>${t("ede.site")}</button></div>`;
  return `<span class="menu-host"><button type="button" class="quiet" data-act="export-menu" aria-expanded="${state.menu === "export"}">${t("export")}</button>${menu}</span>`;
}

function toolbar(state, devices) {
  const searching = state.scan.state === "running";
  const search = `<input id="device-filter" type="search" placeholder="${t("filter.devices")}" value="${h(state.deviceFilter)}" autocomplete="off">`;
  return `<div class="toolbar">${search}${progress(state.scan)}
    <div class="end">
      ${state.selectedDevice !== null ? `<button type="button" class="tonal" data-act="find"${searching ? " disabled" : ""}>${t("searchAgain")}</button>` : ""}
      <span class="menu-host"><button type="button" class="quiet" data-act="locate-menu" aria-expanded="${state.menu === "locate"}">${t("locate.short")}</button>${locateMenu(state)}</span>
      ${devices.length ? exportMenu(state, searching) : ""}
    </div></div>`;
}

/** The device's object count, or how far the search has read its list. */
function objectsCell(device) {
  const objects = device.objects;
  if (!objects) return "";
  if (objects.error) return `<span class="chip warn" title="${h(errorText(objects.error))}">${t("dev.objectsFailed")}</span>`;
  if (objects.done) return String(ownObjects(objects.total));
  return objects.total ? `${ownObjects(objects.read)} / ${ownObjects(objects.total)}` : "…";
}

function rowHtml(state, device) {
  // behind a router, network and MAC tell the devices apart; the router stays in the tooltip
  const station = device.network !== null ? `${t("net")} ${device.network} · MAC ${h(device.mac)}` : "";
  const where = station ? `<span class="mono cut" title="${t("behindRouter", { address: h(device.address) })} · ${station}">${station}</span>` : `<span class="mono cut">${h(device.address)}</span>`;
  const active = state.selectedDevice === device.instance ? " active" : "";
  return `<div class="row devices${active}" data-device="${device.instance}" role="option" aria-selected="${Boolean(active)}">
    <span class="name">${h(deviceName(device))}</span><span class="mono">#${device.instance}</span>
    ${where}<span class="cut">${h(deviceMaker(device) || t("unknownVendor"))}</span>
    <span class="mono objects"><span class="label">${t("col.objects")}: </span>${objectsCell(device)}</span></div>`;
}

function emptyHtml(state) {
  const scan = state.scan;
  if (scan.state === "idle") return `<div class="empty"><h2>${t("dev.idleTitle")}</h2><p>${t("start.lead")}</p></div>`;
  if (scan.state === "running") return `<div class="empty"><p>${t("phase.whoIs")} …</p></div>`;
  if (scan.state === "failed") return "";
  return `<div class="empty"><h2>${t("empty.title")}</h2><p>${t("empty.searched", { network: `<span class="mono">${h(searchScope(state))}</span>` })}</p>
    <div class="actions"><button type="button" class="quiet" data-act="network">${t("empty.otherSubnet")}</button><button type="button" class="quiet" data-act="focus-adapter">${t("empty.otherAdapter")}</button></div></div>`;
}

/** Restarting and stopping a device stops what it controls, so each says so and takes the unlock. */
const COMMANDS = [["warm", "cmd.warm"], ["cold", "cmd.cold"], ["stop", "cmd.stop"], ["resume", "cmd.resume"]];

function commandsHtml(state) {
  const asking = COMMANDS.find(([kind]) => state.asking === `cmd:${kind}`);
  const buttons = COMMANDS.map(([kind, label]) => `<button type="button" class="quiet" data-act="ask-command" data-command="${kind}">${t(label)}</button>`).join("");
  if (!asking) return `<div class="actions">${buttons}</div>`;
  const [kind, label] = asking;
  if (!state.writeUnlocked) {
    return `<div class="ask"><b>${t("unlock.title")}</b><span>${t(`cmd.${kind}Why`)}</span>
      <div class="actions"><button type="button" data-act="unlock-command">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
  }
  const minutes = kind === "stop" ? `<label class="form-field">${t("cmd.minutes")}<input id="cmd-minutes" class="mono" inputmode="numeric" value="${h(state.commandMinutes)}"></label>` : "";
  return `<div class="ask"><b>${t(label)}?</b><span>${t(`cmd.${kind}Why`)}</span>${minutes}
    <label class="form-field">${t("cmd.password")}<input id="cmd-password" type="password" autocomplete="off" value="${h(state.commandPassword)}"></label>
    <div class="actions"><button type="button" data-act="run-command" data-command="${kind}">${t(label)}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
}

/** How many active alarms the device details list before *Show all*. */
const ALARMS_SHOWN = 5;

function alarmsHtml(state) {
  const alarms = state.deviceAlarms;
  const asking = typeof state.asking === "string" && state.asking.startsWith("ack:") ? state.asking.split(":") : null;
  const ask = !asking ? "" : `<div class="ask"><b>${t(state.writeUnlocked ? "alarms.ack" : "unlock.title")}</b><span>${t("alarms.ackWhy")}</span>
    <div class="actions"><button type="button" data-act="unlock-ack" data-type="${asking[1]}" data-instance="${asking[2]}">${t(state.writeUnlocked ? "alarms.ack" : "unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
  let body;
  if (alarms === null) body = `<p class="hint">${t("alarms.reading")}</p>`;
  else if (alarms.error) body = `<p class="hint">${h(errorText(alarms.error))}</p>`;
  else if (!alarms.length) body = `<p class="hint">${t("alarms.none")}</p>`;
  else {
    // a plant in trouble lists dozens: the first few, the rest on request
    const shown = state.alarmsAll ? alarms : alarms.slice(0, ALARMS_SHOWN);
    const open = alarms.filter((alarm) => alarm.unacknowledged).length;
    const summary = open ? `<p class="hint">${t("alarms.summary", { open, n: alarms.length })}</p>` : "";
    const toggle = alarms.length > ALARMS_SHOWN
      ? `<button type="button" class="quiet" data-act="alarms-all">${state.alarmsAll ? t("alarms.fewer") : t("alarms.more", { n: alarms.length })}</button>` : "";
    body = `${summary}<div class="alarms">${shown.map((alarm) => alarmHtml(state, alarm)).join("")}</div>${toggle}`;
  }
  const count = Array.isArray(alarms) && alarms.length ? ` <span class="n">${alarms.length}</span>` : "";
  return `<h3>${t("alarms.title")}${count}</h3>${body}${ask}`;
}

/** The name the device's points gave it, where they have been read; else type and number. */
function alarmName(state, alarm) {
  const points = state.sitePoints[state.selectedDevice]?.points ?? (state.device?.instance === state.selectedDevice ? state.points : null) ?? [];
  const point = points.find((item) => item.type === alarm.type && item.instance === alarm.instance);
  return alarm.name || point?.name || objectText(alarm);
}

function alarmHtml(state, alarm) {
  // where it can be acknowledged, the button says enough that it is not yet
  const ack = alarm.unacknowledged && alarm.can_acknowledge
    ? `<button type="button" class="quiet" data-act="alarm-ack" data-type="${alarm.type}" data-instance="${alarm.instance}">${t("alarms.ack")}</button>` : "";
  return `<div class="alarm"><span class="cut" title="${h(objectText(alarm))}">${h(alarmName(state, alarm))}</span>
    <span class="chips"><span class="chip ${alarm.state === "fault" ? "err" : "warn"}">${h(eventStateText(alarm.state))}</span>${ack || (alarm.unacknowledged ? `<span class="chip">${t("alarms.unacked")}</span>` : "")}</span></div>`;
}

/** Every request of the device's last reading, with its time and outcome: where time goes. */
function requestsHtml(requests) {
  if (!requests.length) return `<p class="hint">${t("req.empty")}</p>`;
  const rows = requests.map((request) => `<dt class="mono">${(request.at_ms / 1000).toFixed(1)} s</dt>
    <dd>${h(request.what)} · <span class="mono">${request.took_ms} ms</span>${request.failed ? ` <span class="chip err" title="${h(request.failed)}">${t("req.failed")}</span>` : ""}</dd>`).join("");
  return `<h3>${t("req.title")}</h3><dl class="facts requests">${rows}</dl>`;
}

/** Setting the clock changes what the device's schedules do, so it takes the unlock and a second word. */
function clockHtml(state) {
  if (state.asking !== "clock") return `<button type="button" class="quiet" data-act="ask-clock">${t("clock.set")}</button>`;
  if (state.writeUnlocked) {
    return `<div class="ask"><b>${t("clock.confirm")}</b><span>${t("clock.why")}</span>
    <div class="actions"><button type="button" data-act="set-clock">${t("clock.set")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
  }
  return `<div class="ask"><b>${t("unlock.title")}</b><span>${t("clock.why")}</span>
    <div class="actions"><button type="button" data-act="unlock-clock">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
}

/** A new name for the device: only a text, but other systems may look for it, so it takes the unlock. */
function nameHtml(state, device) {
  if (state.asking !== "name") return `<button type="button" class="quiet" data-act="ask-name">${t("name.change")}</button>`;
  if (!state.writeUnlocked) {
    return `<div class="ask"><b>${t("unlock.title")}</b><span>${t("name.why")}</span>
      <div class="actions"><button type="button" data-act="unlock-name">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
  }
  return `<div class="ask"><b>${t("name.title", { n: device.instance })}</b><span>${t("name.why")}</span>
    <label class="form-field">${t("name.new")}<input id="new-name" value="${h(state.newName)}"></label>
    <div class="actions"><button type="button" data-act="change-name">${t("name.change")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
}

/** A new device ID takes the unlock and a clear word about what follows. */
function instanceHtml(state, device) {
  if (state.asking !== "instance") return `<button type="button" class="quiet" data-act="ask-instance">${t("instance.change")}</button>`;
  if (!state.writeUnlocked) {
    return `<div class="ask"><b>${t("unlock.title")}</b><span>${t("instance.why")}</span>
      <div class="actions"><button type="button" data-act="unlock-instance">${t("unlock.allow")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
  }
  return `<div class="ask"><b>${t("instance.title", { n: device.instance })}</b><span>${t("instance.why")}</span>
    <label class="form-field">${t("instance.new")}<input id="new-instance" class="mono" inputmode="numeric" value="${h(state.newInstance)}"></label>
    <div class="actions"><button type="button" data-act="change-instance">${t("instance.change")}</button><button type="button" class="quiet" data-act="cancel-write">${t("cancel")}</button></div></div>`;
}

export function deviceDetailHtml(state) {
  const device = state.scan.devices?.find((found) => found.instance === state.selectedDevice);
  if (!device) return `<aside class="detail"><p class="hint">${t("dev.choose")}</p></aside>`;
  const facts = (state.deviceFacts ?? []).map(([label, value]) => `<dt>${h(label)}</dt><dd>${h(value)}</dd>`).join("");
  const route = device.network !== null ? `<dt>${t("net")}</dt><dd class="mono">${device.network} · MAC ${h(device.mac)}</dd>` : "";
  // rarely needed and changing the plant: open when asked for, or while one of them is under way
  const commandsOpen = state.commandsOpen || (typeof state.asking === "string" && !state.asking.startsWith("ack:") && state.asking !== "properties");
  return `<aside class="detail${state.detailOpen ? " open" : ""}" data-keep="device-detail">
    <button type="button" class="quiet back" data-act="close-detail">← ${t("back")}</button>
    <div class="dhead"><b>${h(deviceName(device))}</b><span class="sub">${h(deviceMaker(device))}</span></div>
        <dl class="facts"><dt>${t("dev.id")}</dt><dd class="mono">${device.instance}</dd><dt>${t("dev.address")}</dt><dd class="mono">${h(device.address)}</dd>${route}${device.objects?.error ? `<dt>${t("col.objects")}</dt><dd>${h(errorText(device.objects.error))}</dd>` : ""}${facts}</dl>
    ${state.deviceFacts === null ? `<p class="hint">${t("properties.reading")}</p>` : ""}
    ${alarmsHtml(state)}
    <div class="actions"><button type="button" class="quiet" data-act="requests">${t(state.requestLog ? "req.hide" : "req.show")}</button></div>
    <button type="button" class="section-toggle" data-act="toggle-commands" aria-expanded="${commandsOpen}">${t("cmd.title")} <span aria-hidden="true">${commandsOpen ? "▾" : "▸"}</span></button>
    ${commandsOpen ? `<div class="actions">${clockHtml(state)}${nameHtml(state, device)}${instanceHtml(state, device)}</div>${commandsHtml(state)}` : ""}
    ${bbmdHtml(state)}
    ${state.requestLog ? requestsHtml(state.requests) : ""}
  </aside>`;
}

export function viewDevices(state) {
  const scan = state.scan;
  const all = scan.devices ?? [];
  const devices = filterDevices(all, state.deviceFilter);
  const failed = scan.state === "failed" ? errorStrip(errorText(scan.error)) : "";
  const notes = (scan.notes ?? []).map((item) => noteStrip(item)).join("");
  const routers = (scan.routers ?? []).map((router) =>
    `<div class="strip info">${t("router.networks", { address: h(router.address), networks: router.networks.join(", ") })}</div>`).join("");
  const remembered = scan.remembered_ms
    ? `<div class="strip info">${t("site.remembered", { when: h(new Date(scan.remembered_ms).toLocaleString(currentLocale())) })}</div>`
    : "";
  const strips = failed || notes || routers || remembered ? `<div class="strips">${remembered}${failed}${notes}${routers}</div>` : "";
  const sort = state.deviceSort;
  const head = `<div class="row devices head" role="row">${sortHead(t("col.device"), "devices", "name", sort)}${sortHead(t("col.id"), "devices", "id", sort)}${sortHead(t("col.address"), "devices", "address", sort)}${sortHead(t("col.vendor"), "devices", "vendor", sort)}${sortHead(t("col.objects"), "devices", "objects", sort, "objects")}</div>`;
  const rows = devices.length ? head + sortRows(devices, sort, DEVICE_COLUMNS).map((device) => rowHtml(state, device)).join("") : all.length ? `<div class="empty"><p>${t("filter.none")}</p></div>` : emptyHtml(state);
  const lead = scan.state === "done" ? t("dev.leadDone", { count: deviceCount(all.length), scope: h(searchScope(state)) }) : t("dev.lead");
  return {
    title: t("tab.devices"),
    lead,
    html: `<section class="listwrap">${listBorders()}${toolbar(state, all)}${strips}<div class="rows" data-keep="devices" tabindex="0" role="listbox" data-list="devices">${rows}</div></section>
      ${deviceDetailHtml(state)}`,
  };
}
