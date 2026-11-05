// The plugin's page: the devices with their profile, the check one after the other, the results
// per rule, the marks in bacnetio's lists and the report. Talks to bacnetio only through `ask`.

import { RULES, checkDevice, countNames, defaultRules, sourceOf } from "./check.js";
import { PROFILES, SOURCE } from "./rules.js";
import { TEXTS } from "./texts.js";

const DEVICE_RULES = ["revision", "objectTypes", "services"];
const SHOWN_OBJECTS = 50;

let next = 0;
const waiting = new Map();
addEventListener("message", (event) => {
  if (event.source !== parent) return;
  const { call, result, error } = event.data ?? {};
  const asked = waiting.get(call);
  if (!asked) return;
  waiting.delete(call);
  if (error) asked.reject(new Error(error));
  else asked.resolve(result);
});
const ask = (method, ...args) => new Promise((resolve, reject) => {
  const call = ++next;
  waiting.set(call, { resolve, reject });
  parent.postMessage({ call, method, args }, "*");
});

let texts = TEXTS.en;
let locale = "en-GB";
const t = (key, values = {}) => String(key.split(".").reduce((part, name) => part?.[name], texts) ?? key)
  .replace(/\{(\w+)\}/g, (_, name) => values[name] ?? "");
const h = (text) => String(text ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

/** Kept in bacnetio's data folder: profiles chosen, results and object names per device. */
let kept = { profiles: {}, devices: {} };
let devices = [];
let busy = null;
let stopAsked = false;
const open = new Set();

const profileOf = (instance) => kept.profiles[instance] ?? "AS-CH";

/** Everything bacnetio reads of a device for the check: its objects and each one's properties. */
async function readDevice(instance, progress) {
  const objects = await ask("objects", instance);
  const read = [];
  for (const [index, object] of objects.entries()) {
    if (stopAsked) throw new Error("stopped");
    progress(index, objects.length);
    let properties = null;
    try {
      properties = await ask("properties", instance, object.type, object.instance);
    } catch {
      // left open in the report, not counted as met
    }
    read.push({ type: object.type, instance: object.instance, name: object.name, properties });
  }
  return { instance, objects: read };
}

/** Object names of every device checked so far, for the rule that a name is used once (5.3.3). */
function names(extra = []) {
  const known = Object.entries(kept.devices)
    .filter(([instance]) => !extra.some((device) => String(device.instance) === instance))
    .map(([, entry]) => ({ objects: (entry.names ?? []).map((name) => ({ name })) }));
  return countNames([...known, ...extra]);
}

async function check(instances) {
  stopAsked = false;
  const read = [];
  for (const instance of instances) {
    busy = { instance, done: 0, total: 0 };
    render();
    try {
      read.push(await readDevice(instance, (done, total) => { busy = { instance, done, total }; render(); }));
    } catch (error) {
      if (stopAsked) break;
      kept.devices[instance] = { at: Date.now(), error: error.message };
    }
  }
  // names compare across all devices read, so the results come once everything is in
  const counted = names(read);
  for (const device of read) {
    kept.devices[device.instance] = {
      at: Date.now(),
      result: checkDevice(device, profileOf(device.instance), counted, { rules: kept.rules }),
      names: device.objects.map((object) => object.name).filter(Boolean),
    };
  }
  busy = null;
  await save();
  render();
}

async function save() {
  await ask("save", kept);
  await ask("mark", marks());
}

function findingText(finding) {
  return t(`findings.${finding.rule}`, { what: finding.what.join(", ") });
}

const MET = { label: "KBOB ✓", state: "ok" };
const NOT_MET = { label: "KBOB ✗", state: "bad" };
const OPEN = { label: "KBOB ?" };

/** The result in bacnetio's lists: ✗ with what is wrong, ✓ where all is met, on devices and objects. */
function marks() {
  const all = [];
  for (const [instance, entry] of Object.entries(kept.devices)) {
    const result = entry.result;
    if (!result) continue;
    const device = Number(instance);
    const rules = Object.values(result.rules);
    const notMet = rules.filter((rule) => rule.ok === false).length;
    const open = rules.filter((rule) => rule.ok === null).length;
    if (notMet) all.push({ device, ...NOT_MET, text: t("markDevice", { n: notMet }) });
    else all.push({ device, ...(open ? OPEN : MET), text: t(open ? "markOpen" : "markMet") });
    for (const object of result.objects) {
      const what = object.findings.map((finding) => `${findingText(finding)} (${sourceOf(finding.rule)})`).join("; ");
      all.push({ device, type: object.type, instance: object.instance, ...NOT_MET, text: t("mark", { what }) });
    }
    for (const [type, objectInstance] of result.passed ?? []) {
      all.push({ device, type, instance: objectInstance, ...MET, text: t("markMet") });
    }
  }
  return all;
}

/** The report for people, as the owner gets it: in the interface's language, for Excel. */
function report() {
  const quote = (value) => `"${String(value ?? "").replace(/"/g, '""')}"`;
  const word = (state) => (state.ok === true ? t("ruleMet") : state.ok === false ? t("ruleNotMet") : t("ruleOpen"));
  const rows = [["device", "name", "profile", "checked", "rule", "source", "result", "object", "finding"].map((key) => t(`csv.${key}`))];
  for (const device of devices) {
    const entry = kept.devices[device.instance];
    if (!entry?.result) continue;
    const base = [device.instance, device.name, entry.result.profile, new Date(entry.at).toLocaleString(locale)];
    for (const rule of RULES.filter((name) => entry.result.rules[name])) {
      const state = entry.result.rules[rule];
      const head = [...base, t(`rules.${rule}`), sourceOf(rule)];
      const deviceFinding = DEVICE_RULES.includes(rule) && state.found?.length ? state.found.join(", ") : "";
      rows.push([...head, word(state), "", deviceFinding]);
      for (const object of entry.result.objects) {
        for (const finding of object.findings.filter((candidate) => candidate.rule === rule)) {
          rows.push([...head, t("ruleNotMet"), object.label, finding.what.join(", ")]);
        }
      }
    }
  }
  // the mark at the start lets Excel read the umlauts
  ask("download", "kbob-report.csv", `\uFEFF${rows.map((row) => row.map(quote).join(";")).join("\r\n")}\r\n`);
}

function summary(entry) {
  if (!entry) return `<span class="muted">${t("notChecked")}</span>`;
  if (entry.error) return `<span class="bad">${h(t("readFailed", { error: entry.error }))}</span>`;
  const rules = Object.values(entry.result.rules);
  const notMet = rules.filter((rule) => rule.ok === false).length;
  const openRules = rules.filter((rule) => rule.ok === null).length;
  const text = notMet ? `<span class="bad">${t("notMet", { n: notMet, total: rules.length })}</span>` : `<span class="good">${t("allMet")}</span>`;
  return `${text}${openRules ? ` · <span class="muted">${t("open", { n: openRules })}</span>` : ""}`;
}

function ruleRow(rule, state, objects) {
  const word = state.ok === true ? `<span class="good">✓ ${t("ruleMet")}</span>` : state.ok === false ? `<span class="bad">✗ ${t("ruleNotMet")}</span>` : `<span class="muted">? ${t("ruleOpen")}</span>`;
  const found = DEVICE_RULES.includes(rule) && state.found?.length ? `<div class="found">${h(t(`findings.${rule}`, { what: state.found.join(", ") }))}</div>` : "";
  const failing = objects.filter((object) => object.findings.some((finding) => finding.rule === rule));
  const list = failing.slice(0, SHOWN_OBJECTS).map((object) =>
    `<li><b>${h(object.label)}</b> ${h(object.findings.filter((finding) => finding.rule === rule).map(findingText).join("; "))}</li>`).join("");
  const more = failing.length > SHOWN_OBJECTS ? `<li class="muted">${t("more", { n: failing.length - SHOWN_OBJECTS })}</li>` : "";
  return `<li class="rule"><div class="rule-head"><span>${h(t(`rules.${rule}`))} <span class="muted">(${sourceOf(rule)})</span></span>${word}</div>${found}${failing.length ? `<ul class="objects">${list}${more}</ul>` : ""}</li>`;
}

/** Which rules a check takes; AKS is off until chosen, as it is an example of the guide. */
let chooserOpen = false;
function chooserHtml() {
  const items = RULES.map((rule) => `<li><label class="option"><input type="checkbox" data-act="rule" data-rule="${rule}"${kept.rules.includes(rule) ? " checked" : ""}${busy ? " disabled" : ""}>
    ${h(t(`rules.${rule}`))} <span class="muted">(${sourceOf(rule)})</span></label>${rule === "aks" ? `<p class="muted small">${t("aksNote")}</p>` : ""}</li>`).join("");
  return `<details class="chooser"${chooserOpen ? " open" : ""}><summary>${t("chooseRules", { n: kept.rules.length, total: RULES.length })}</summary>
    <ul class="choices">${items}</ul><p class="muted small">${t("chooseNote")}</p></details>`;
}

function deviceRow(device) {
  const entry = kept.devices[device.instance];
  const profiles = PROFILES.map((profile) => `<option${profile === profileOf(device.instance) ? " selected" : ""}>${profile}</option>`).join("");
  const progress = busy?.total ? t("reading", { done: busy.done + 1, total: busy.total }) : "…";
  const reading = busy?.instance === device.instance ? `<span class="muted">${progress}</span>` : summary(entry);
  const expandable = Boolean(entry?.result);
  const expanded = expandable && open.has(device.instance);
  const details = expanded ? `<tr class="details"><td colspan="4"><ul class="rules">${RULES.filter((rule) => entry.result.rules[rule]).map((rule) => ruleRow(rule, entry.result.rules[rule], entry.result.objects)).join("")}</ul></td></tr>` : "";
  return `<tr>
    <td><b>${h(device.name ?? `#${device.instance}`)}</b> <span class="muted mono">#${device.instance}</span></td>
    <td><select data-act="profile" data-device="${device.instance}"${busy ? " disabled" : ""}>${profiles}</select></td>
    <td>${reading}${entry?.at && !busy ? ` <span class="muted">· ${t("checkedAt", { time: new Date(entry.at).toLocaleString(locale) })}</span>` : ""}</td>
    <td class="acts">${expandable ? `<button data-act="toggle" data-device="${device.instance}" aria-expanded="${expanded}">${expanded ? t("hideDetails") : t("details")}</button>` : ""}<button data-act="check" data-device="${device.instance}"${busy ? " disabled" : ""}>${entry ? t("checkAgain") : t("check")}</button></td></tr>${details}`;
}

function render() {
  const app = document.getElementById("app");
  if (!devices.length) {
    app.innerHTML = `<p class="muted">${t("noDevices")}</p>`;
    return;
  }
  const anyResult = Object.values(kept.devices).some((entry) => entry.result);
  app.innerHTML = `<div class="bar">
      ${busy ? `<button data-act="stop">${t("stop")}</button>` : `<button class="primary" data-act="check-all">${t("checkAll")}</button>`}
      <button data-act="report"${anyResult && !busy ? "" : " disabled"}>${t("report")}</button>
      <button data-act="clear"${anyResult && !busy ? "" : " disabled"}>${t("clear")}</button>
    </div>
    ${chooserHtml()}
    <table><thead><tr><th>${t("device")}</th><th>${t("profile")}</th><th>${t("result")}</th><th></th></tr></thead>
    <tbody>${devices.map(deviceRow).join("")}</tbody></table>
    <p class="muted small">${t("namesNote")}</p>
    <details class="limits"><summary>${t("notChecked_title")}</summary><p class="muted small">${t("notChecked_list")}</p></details>
    <p class="muted small">${h(SOURCE)}</p>`;
}

document.addEventListener("click", async (event) => {
  const button = event.target.closest("button[data-act]");
  if (!button) return;
  const instance = Number(button.dataset.device);
  const acts = {
    "check-all": () => check(devices.map((device) => device.instance)),
    check: () => check([instance]),
    stop: () => { stopAsked = true; },
    report: () => report(),
    clear: async () => { kept.devices = {}; await save(); render(); },
    toggle: () => { if (open.has(instance)) open.delete(instance); else open.add(instance); render(); },
  };
  await acts[button.dataset.act]?.();
});

document.addEventListener("change", async (event) => {
  if (event.target.dataset.act === "rule") {
    const rule = event.target.dataset.rule;
    kept.rules = RULES.filter((name) => (name === rule ? event.target.checked : kept.rules.includes(name)));
    chooserOpen = true;
    await ask("save", kept);
    render();
    return;
  }
  if (event.target.dataset.act !== "profile") return;
  kept.profiles[event.target.dataset.device] = event.target.value;
  await ask("save", kept);
});

async function start() {
  const context = await ask("context");
  texts = TEXTS[context.language] ?? TEXTS.en;
  document.documentElement.lang = context.language;
  for (const [name, value] of Object.entries(context.tokens)) if (value) document.documentElement.style.setProperty(name, value);
  locale = context.language === "de" ? "de-CH" : "en-GB";
  document.getElementById("lead").textContent = t("lead");
  kept = { profiles: {}, devices: {}, ...(await ask("load") ?? {}) };
  // the rules chosen; before there was a choice, only AKS could be switched on
  kept.rules ??= kept.aks ? [...defaultRules(), "aks"] : defaultRules();
  delete kept.aks;
  devices = await ask("devices");
  render();
  await ask("mark", marks());
}

start().catch((error) => {
  document.getElementById("app").textContent = error.message;
});
