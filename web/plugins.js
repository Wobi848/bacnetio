// Plugins in the interface (docs/concept-plugins.md): the card in the settings, the entries in the
// sidebar, a plugin's page and the marks plugins put on devices and objects.

import { priorityName } from "./format.js";
import { h } from "./html.js";
import { t } from "./i18n.js";

export const PLUGIN_AREA = "plugin/";
const PLUGIN_ICON = '<svg class="nav-i" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 4a2 2 0 1 1 4 0v2h4v4h-2a2 2 0 1 0 0 4h2v4h-4v-2a2 2 0 1 0-4 0v2H6v-4h2a2 2 0 1 0 0-4H6V6h4z"/></svg>';

export const isPluginArea = (area) => area.startsWith(PLUGIN_AREA);
export const pluginOfArea = (state) => (isPluginArea(state.area) ? state.plugins.find((plugin) => `${PLUGIN_AREA}${plugin.id}` === state.area && plugin.on) : null);

/** One entry per plugin switched on, below the areas. */
export function pluginTabsHtml(state) {
  return state.plugins.filter((plugin) => plugin.on).map((plugin) => {
    const area = `${PLUGIN_AREA}${plugin.id}`;
    const current = state.area === area ? ' aria-current="page"' : "";
    return `<a href="#/${area}" data-area="${area}"${current} title="${h(plugin.description || plugin.name)}">${PLUGIN_ICON}<span>${h(plugin.name)}</span></a>`;
  }).join("");
}

/** The plugin's own page runs in a frame beside the work area (plugin-frames.js); this is its head. */
export function viewPlugin(state) {
  const plugin = pluginOfArea(state);
  if (!plugin) return { title: t("plugins.title"), lead: "", next: null, html: `<div class="split single"><section class="listwrap"><div class="rows"><div class="empty"><p>${t("plugins.gone")}</p></div></div></section></div>` };
  // the plugin's page says what it does in the interface's language; its description is one language
  return { title: plugin.name, lead: "", next: null, html: "" };
}

function pluginRow(state, plugin) {
  const name = `<b>${h(plugin.name)}</b> <span class="sub mono">${h(plugin.version)}</span>`;
  if (state.pluginRemoving === plugin.id) {
    return `<li class="plugin">${name}<p>${t("plugins.removeAsk", { name: h(plugin.name) })}</p>
      <span class="acts"><button type="button" data-act="plugin-remove-yes" data-id="${h(plugin.id)}">${t("plugins.remove")}</button><button type="button" class="quiet" data-act="plugin-remove-no">${t("cancel")}</button></span></li>`;
  }
  return `<li class="plugin">${name}${plugin.description ? `<p class="hint">${h(plugin.description)}</p>` : ""}
    <span class="acts"><span class="seg"><button type="button" data-act="plugin-switch" data-id="${h(plugin.id)}" data-on="true" aria-pressed="${plugin.on}">${t("update.on")}</button><button type="button" data-act="plugin-switch" data-id="${h(plugin.id)}" data-on="false" aria-pressed="${!plugin.on}">${t("update.off")}</button></span>
    <button type="button" class="quiet" data-act="plugin-remove" data-id="${h(plugin.id)}">${t("plugins.remove")}</button></span></li>`;
}

function replaceHtml(state) {
  const asked = state.pluginReplacing;
  if (!asked) return "";
  return `<div class="strip warn">${t("plugins.replaceAsk", { name: h(asked.name), version: h(asked.version) })}
    <span class="acts"><button type="button" data-act="plugin-replace-yes">${t("plugins.replace")}</button><button type="button" class="quiet" data-act="plugin-replace-no">${t("cancel")}</button></span></div>`;
}

/** Settings → Extensions. */
export function pluginsCardHtml(state) {
  const list = state.plugins.length ? `<ul class="plugins">${state.plugins.map((plugin) => pluginRow(state, plugin)).join("")}</ul>` : `<p class="hint">${t("plugins.none")}</p>`;
  return `<section class="card"><h2>${t("plugins.title")}</h2>
    ${list}${replaceHtml(state)}
    <div class="acts"><label class="button">${t("plugins.install")}<input id="plugin-file" type="file" accept=".html,text/html" hidden></label></div>
    <p class="hint">${t("plugins.safety")}</p></section>`;
}

/** The marks of every plugin switched on, for a device (no type) or one of its objects. */
export function pluginMarksHtml(state, device, type = null, instance = null) {
  const shown = state.plugins.filter((plugin) => plugin.on).flatMap((plugin) =>
    (state.marks[plugin.id] ?? [])
      .filter((mark) => mark.device === device && (type === null ? mark.type === undefined : mark.type === type && mark.instance === instance))
      .map((mark) => ({ plugin, mark })));
  return shown.map(({ plugin, mark }) =>
    `<span class="chip mark${mark.state ? ` ${mark.state}` : ""}" title="${h(plugin.name)}: ${h(mark.text)}">${h(mark.label ?? plugin.name)}</span>`).join("");
}

/** A priority's name: a plugin's (a standard's use of it) before BACnet's own, and which plugin gave it. */
export function priorityLabel(state, priority) {
  for (const plugin of state.plugins.filter((entry) => entry.on)) {
    const name = state.priorityNames[plugin.id]?.[priority];
    if (name) return { name, by: plugin.name };
  }
  return { name: priorityName(priority), by: "" };
}
