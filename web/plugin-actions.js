// Plugins: installing, switching and removing them, and the bridge that answers a plugin's
// questions (plugin API version 1, docs/plugins.md). A plugin can only ask; it never reaches the
// server itself, and nothing here writes to the plant.

import { api } from "./api.js";
import { t } from "./i18n.js";
import { pluginOfWindow } from "./plugin-frames.js";
import { render, state, toast } from "./store.js";

/** bacnetio's colours and fonts, so a plugin looks like the rest. */
const STYLE_TOKENS = ["--bg", "--surface", "--sunken", "--line", "--line-strong", "--text", "--muted", "--accent", "--accent-ink", "--accent-soft", "--ok", "--ok-soft", "--warn", "--warn-soft", "--err", "--err-soft", "--mono"];
/** A plugin points at what it found; more than this is a list, not a mark. */
const MOST_MARKS = 20000;
const MOST_MARK_CHARS = 200;

export async function loadPlugins() {
  try {
    state.plugins = await api.plugins();
  } catch (error) {
    // without a data folder (some tests) there are no plugins; anything else is said
    state.plugins = [];
    if (error.status !== 409) toast(error.message, true);
  }
}

/** Name and version of a plugin about to replace another; the server has checked the page already. */
function newcomer(page) {
  const description = /id="bacnetio-plugin">([\s\S]*?)<\/script>/.exec(page);
  try {
    const { name = "", version = "" } = JSON.parse(description?.[1] ?? "{}");
    return { name, version };
  } catch {
    return { name: "", version: "" };
  }
}

export async function installPlugin(file, replace = false) {
  const page = typeof file === "string" ? file : await file.text();
  try {
    const plugin = await api.installPlugin(page, replace);
    state.pluginReplacing = null;
    toast(t("plugins.installed", { name: plugin.name, version: plugin.version }));
  } catch (error) {
    if (error.status !== 409) {
      toast(error.message, true);
      return;
    }
    state.pluginReplacing = { page, ...newcomer(page) };
  }
  await loadPlugins();
  render();
}

export async function switchPlugin(id, on) {
  try {
    await api.switchPlugin(id, on);
  } catch (error) {
    toast(error.message, true);
  }
  // a plugin switched off takes its marks with it
  if (!on) delete state.marks[id];
  await loadPlugins();
  render();
}

export async function removePlugin(id) {
  try {
    await api.removePlugin(id);
  } catch (error) {
    toast(error.message, true);
  }
  delete state.marks[id];
  state.pluginRemoving = null;
  await loadPlugins();
  render();
}

function context(id) {
  const style = getComputedStyle(document.documentElement);
  const theme = document.documentElement.dataset.theme;
  const tokens = Object.fromEntries(STYLE_TOKENS.map((name) => [name, style.getPropertyValue(name).trim()]));
  return {
    plugin: id,
    version: state.version,
    language: document.documentElement.lang,
    dark: theme === "dark" || (theme !== "light" && matchMedia("(prefers-color-scheme: dark)").matches),
    tokens,
    device: state.device?.instance ?? null,
  };
}

const devices = () => (state.scan.devices ?? []).map(({ instance, name, address, network, mac, vendor, model }) => ({ instance, name, address, network, mac, vendor, model }));

/** Only what a mark needs, checked: a plugin's message is not trusted. */
function setMarks(id, marks) {
  if (!Array.isArray(marks)) throw new Error("marks must be a list");
  const number = (value) => Number.isInteger(value) && value >= 0;
  state.marks[id] = marks.slice(0, MOST_MARKS)
    .filter((mark) => number(mark?.device) && typeof mark.text === "string")
    .map((mark) => ({
      device: mark.device,
      ...(number(mark.type) && number(mark.instance) ? { type: mark.type, instance: mark.instance } : {}),
      text: mark.text.slice(0, MOST_MARK_CHARS),
    }));
  render();
  return state.marks[id].length;
}

function download(name, text) {
  const link = document.createElement("a");
  link.href = URL.createObjectURL(new Blob([String(text)], { type: "text/plain;charset=utf-8" }));
  link.download = String(name).replace(/[\\/:*?"<>|]/g, "_");
  link.click();
  URL.revokeObjectURL(link.href);
  return null;
}

const number = (value, what) => {
  if (!Number.isInteger(value) || value < 0) throw new Error(`${what} must be a whole number`);
  return value;
};

/** What a plugin may ask: reading and its own things, never a write to the plant. */
const QUESTIONS = {
  context: (id) => context(id),
  devices: () => devices(),
  objects: (id, [device]) => api.points(number(device, "device")).then((reading) => reading.points),
  properties: (id, [device, type, instance]) => api.properties(number(device, "device"), number(type, "type"), number(instance, "instance")),
  load: (id) => api.pluginStore(id),
  save: (id, [data]) => api.savePluginStore(id, data),
  download: (id, [name, text]) => download(name, text),
  mark: (id, [marks]) => setMarks(id, marks),
};

/** A plugin opened by a reload starts at once; it gets answers once bacnetio has its own state back. */
let markReady;
const ready = new Promise((resolve) => { markReady = resolve; });
export const pluginsMayAsk = () => markReady();

async function answer(event) {
  const id = pluginOfWindow(event.source);
  if (!id) return;
  await ready;
  const { call, method, args } = event.data ?? {};
  const reply = (message) => event.source.postMessage({ call, ...message }, "*");
  const question = Object.hasOwn(QUESTIONS, method) ? QUESTIONS[method] : null;
  if (!question) {
    reply({ error: `unknown question ${String(method)}` });
    return;
  }
  try {
    reply({ result: await question(id, Array.isArray(args) ? args : []) });
  } catch (error) {
    reply({ error: error.message });
  }
}

export function installPluginBridge() {
  window.addEventListener("message", answer);
}
