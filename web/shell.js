// The frame around every page: sidebar (network, areas, settings), page head and status bar.

import { captureChipHtml } from "./capture.js";
import { h } from "./html.js";
import { t } from "./i18n.js";
import { heardHtml, linkChoiceHtml, mstpActsHtml, mstpFieldsHtml, trunkHtml } from "./mstp.js";
import { resizeHandle } from "./panels.js";
import { pluginTabsHtml } from "./plugins.js";
import { siteProblems } from "./problems.js";
import { openSiteOf, siteName } from "./sites.js";

const ICON = (path) => `<svg class="nav-i" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${path}</svg>`;
export const AREA_ICONS = {
  workspace: ICON('<rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="5" rx="2"/><rect x="13" y="11" width="8" height="10" rx="2"/><rect x="3" y="14" width="8" height="7" rx="2"/>'),
  problems: ICON('<path d="M12 3l9.5 17h-19z"/><path d="M12 10v4.5M12 17.5v.5"/>'),
  watch: ICON('<path d="M2.5 12S6 5 12 5s9.5 7 9.5 7-3.5 7-9.5 7-9.5-7-9.5-7z"/><circle cx="12" cy="12" r="2.5"/>'),
  recording: ICON('<path d="M3 17l5-6 4 3 5-7 4 4"/><path d="M3 21h18"/>'),
  log: ICON('<path d="M14.5 4.5l5 5L9 20H4v-5z"/><path d="M12 7l5 5"/>'),
};
const SETTINGS_ICON = ICON('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>');

const HELP_ICON = ICON('<circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 17.5v.5"/>');

function badge(state, area) {
  if (area === "workspace") return `<span class="n">${state.scan.devices?.length || ""}</span>`;
  // a count that means a problem in the warning colour, as the held values (SidebarNav)
  if (area === "problems") return `<span class="n warn">${siteProblems(state).length || ""}</span>`;
  if (area === "watch") return `<span class="n">${state.watch.length || ""}</span>`;
  if (area === "recording") return `<span class="n">${state.recording?.running ? "●" : ""}</span>`;
  if (area === "log") return `<span class="n warn">${state.held.length || ""}</span>`;
  return "";
}

function tab(state, area, key) {
  const current = state.area === area ? ' aria-current="page"' : "";
  return `<a href="#/${area}" data-area="${area}"${current} title="${t(`tab.${area}`)} (${key})">${AREA_ICONS[area]}<span>${t(`tab.${area}`)}</span>${badge(state, area)}</a>`;
}

function connection(state) {
  const scan = state.scan;
  if (scan.state === "running") return `<span class="state"><span class="dot busy"></span>${t("conn.searching")}</span>`;
  if (scan.state === "failed") return `<span class="state"><span class="dot err"></span>${t("conn.failed")}</span>`;
  if (scan.state === "done") {
    const n = scan.devices?.length ?? 0;
    return `<span class="state"><span class="dot on"></span>${n === 1 ? t("side.foundOne") : t("side.found", { n })}</span>`;
  }
  return `<span class="state"><span class="dot"></span>${t("conn.ready")}</span>`;
}

/** Where the search looks, in one line: the trunk bacnetio is on, the subnet entered, or the adapter's. */
export function searchScope(state) {
  const trunk = state.mstp?.trunk;
  if (trunk) return `${trunk.port} · ${trunk.baud}`;
  // a finished search is the site it found: where it looked then, not what the settings say now
  const site = state.scan.state === "done" ? openSiteOf(state) : null;
  if (site) return site.name;
  if (state.settings.subnet.trim()) return state.settings.subnet.trim();
  if (state.settings.bbmd.trim()) return `BBMD ${state.settings.bbmd.trim()}`;
  return state.adapter ? `${state.adapter.ip}/${state.adapter.prefix_len}` : "";
}

/** The network adapter to search on, chosen in the search settings. */
function adapterField(state) {
  if (!state.adapters.length) return `<span class="hint">${t("start.noAdapter")}</span>`;
  const options = state.adapters.map((adapter, index) => {
    const label = `${adapter.name} · ${adapter.ip}`;
    return `<option value="${index}"${adapter === state.adapter ? " selected" : ""}>${h(label)}</option>`;
  }).join("");
  return `<select id="adapter" aria-label="${t("adapter.title")}" title="${h(state.adapter?.description ?? state.adapter?.name ?? "")}">${options}</select>`;
}

/** Search settings (Dialog): where the search looks, with the adapter; on a dimmed page, focus kept inside. */
export function searchDialog(state) {
  if (state.menu !== "network") return "";
  const mstp = state.mstpAvailable && state.link === "mstp";
  const next = mstp ? mstpActsHtml(state) : `<button type="button" data-act="find">${t("searchWithSettings")}</button>`;
  return `<div class="modal"><div class="modal-box search-settings" role="dialog" aria-modal="true" aria-labelledby="net-title">
    <h2 id="net-title">${t("net.title")}</h2>
    <p class="sub">${t("net.lead")}</p>
    <dl class="facts">
      ${linkChoiceHtml(state)}
      ${mstp ? mstpFieldsHtml(state) : ipFields(state)}
    </dl>
    ${mstp ? heardHtml(state) : ""}
    <div class="acts"><button type="button" class="quiet" data-act="close-menu">${t("cancel")}</button>${next}</div>
  </div></div>`;
}

/** The fields of the search settings with BACnet/IP chosen. */
function ipFields(state) {
  const value = (key) => h(state.settings[key]);
  const field = (id, label, input) => `<dt><label for="${id}">${label}</label></dt><dd>${input}</dd>`;
  return `${field("adapter", t("adapter.title"), adapterField(state))}
      <dd class="hint">${t("vpn.hint")}</dd>
      ${field("subnet", t("subnet"), `<input id="subnet" class="mono" placeholder="192.168.10.0/24" value="${value("subnet")}">`)}
      ${field("bbmd", t("bbmd"), `<input id="bbmd" class="mono" placeholder="10.20.0.10" value="${value("bbmd")}">`)}
      ${field("port", t("port"), `<input id="port" class="mono" inputmode="numeric" value="${value("port")}">`)}
      ${field("id-from", t("range.from"), `<input id="id-from" class="mono" inputmode="numeric" placeholder="0" value="${value("idFrom")}">`)}
      ${field("id-to", t("range.to"), `<input id="id-to" class="mono" inputmode="numeric" placeholder="4194302" value="${value("idTo")}">`)}
      <dd class="hint">${t("range.hint")}</dd>`;
}

/** The site as a select field: the known sites, the open one marked, the page of all sites last. */
function siteMenu(state) {
  if (state.menu !== "sites") return "";
  const sites = state.sites.map((site, index) => {
    const name = siteName(site) ?? site.name;
    const act = site.open ? 'data-act="close-menu"' : `data-act="site-open" data-index="${index}"`;
    return `<button type="button" id="site-choice-${index}" class="quiet" role="option" aria-checked="${Boolean(site.open)}" ${act} title="${h(site.name)}"><span class="cut">${h(name)}</span></button>`;
  }).join("");
  return `<div class="menu site-choices" role="listbox" aria-label="${t("side.site")}">${sites}
    <hr role="separator"><button type="button" id="site-choice-all" class="quiet all-sites" role="option" aria-checked="false" data-act="sites-all">${t("side.allSites")}</button></div>`;
}

const CHEVRON = '<svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 9l6 6 6-6"/></svg>';
const SLIDERS_ICON = ICON('<path d="M4 7h10M18 7h2M4 17h2M10 17h10"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/>');

/**
 * The head of the sidebar (SidebarHead): one card with the site as a select field, below it the
 * network and how the search went, and beside them the search settings. The adapter is setup and
 * sits in the search settings, not in the card.
 */
function sideCard(state) {
  const site = openSiteOf(state);
  // a site known only by its network shows that network as its name, said once
  const name = site ? siteName(site) ?? site.name : t("side.noSite");
  const network = site && site.name !== name ? `<span class="site-net mono">${h(site.name)}</span>` : "";
  return `<div class="side-card">
    <section class="side-part site-group" aria-label="${t("side.site")}"><span class="cap">${t("side.site")}</span>
      <div class="menu-host site-host"><button type="button" id="site-switch" class="quiet site-switch" data-act="site-menu" aria-haspopup="listbox" aria-expanded="${state.menu === "sites"}" title="${h(name)}"><span class="site-now">${h(name)}</span>${CHEVRON}</button>${siteMenu(state)}</div>
      <div class="side-meta"><span class="side-detail">${network}${connection(state)}</span>
        <button type="button" id="search-settings" class="quiet side-tool" data-act="network" aria-expanded="${state.menu === "network"}" aria-label="${t("side.searchSettings")}" title="${t("side.searchSettings")}">${SLIDERS_ICON}<span class="side-tool-label">${t("side.searchSettings")}</span></button></div>
      ${trunkHtml(state)}</section>
  </div>`;
}

/** The note in the sidebar: only when a newer release is out. */
export function updateNoteHtml(state) {
  const update = state.update;
  if (!update?.newer) return "";
  return `<a class="update-note" href="${h(update.url)}" target="_blank" rel="noopener">${t("update.new", { version: update.version })}</a>`;
}

export function sideHtml(state) {
  // the keys 1 to 6 choose the areas in this order, settings last (keys.js)
  const listed = ["workspace", "problems", "watch", "recording", "log"];
  const areas = listed.map((area, index) => tab(state, area, index + 1)).join("");
  const settingsCurrent = state.area === "settings" ? ' aria-current="page"' : "";
  const helpCurrent = state.area === "help" ? ' aria-current="page"' : "";
  const open = headOpen(state);
  return `<a class="brand" href="#/workspace"><img src="icon.svg" alt=""><span class="brand-name">bacnetio</span></a>
    ${headToggle(state, open)}
    ${updateNoteHtml(state)}
    <div class="head-more${open ? " open" : ""}" id="head-more">
    ${sideCard(state)}
    <label class="site-search"><input id="site-search" type="search" placeholder="${t("search.site")}" aria-keyshortcuts="/" value="${h(state.siteSearch)}" autocomplete="off"><kbd aria-hidden="true">/</kbd></label>
    </div>
    <nav class="tabs" aria-label="${t("areas")}">${areas}${pluginTabsHtml(state)}</nav>
    <a class="quiet settings-link" href="#/settings" data-area="settings"${settingsCurrent} title="${t("tab.settings")} (${listed.length + 1})">${SETTINGS_ICON}<span class="side-label">${t("tab.settings")}</span></a>
    <a class="quiet settings-link help-link" href="#/help" data-area="help"${helpCurrent} title="${t("help.title")} (?)">${HELP_ICON}<span class="side-label">${t("help.title")}</span></a>
    ${resizeHandle("side")}`;
}

/** On a narrow screen the site, adapter and site search fold away until needed; before the first search they stay open. */
function headOpen(state) {
  return state.headOpen || state.scan.state === "idle" || Boolean(state.siteSearch) || state.menu === "network" || state.menu === "sites";
}

/** The folded head in one line: the site and how many devices answered. Only shown on a narrow screen. */
function headToggle(state, open) {
  const site = openSiteOf(state);
  const name = !site ? t("side.noSite") : siteName(site) ?? site.name;
  return `<button type="button" class="quiet head-toggle" data-act="toggle-head" aria-controls="head-more" aria-expanded="${open}" title="${t("head.toggle")}"><span class="cut">${h(name)}</span>${connection(state)}<span class="caret" aria-hidden="true">${open ? "▴" : "▾"}</span></button>`;
}

/** The tool button beside the next step, again in the toolbar: below 1000 px it moves there. */
export const headToolHtml = (also) => (also ? `<button type="button" class="quiet toolbar-head-tool" data-act="${also.act}">${h(also.label)}</button>` : "");

/** Title, one sentence and the one next step: every page says where you are. */
export function nextbarHtml(view) {
  const next = view.next ? `<button type="button" id="next" data-act="${view.next.act}"${view.next.data ?? ""}>${h(view.next.label)}</button>` : "";
  // a tool beside the next step moves into the toolbar on a phone, unless it stays (no toolbar copy)
  const also = view.also ? `<button type="button" class="quiet head-tool${view.also.stays ? " stays" : ""}" data-act="${view.also.act}">${h(view.also.label)}</button>` : "";
  return `<div class="pagehead"><h1>${h(view.title)}</h1><p class="lead">${view.lead ?? ""}</p></div>${also ? `<span class="next-acts">${also}${next}</span>` : next}`;
}

function activity(state) {
  const scan = state.scan;
  if (scan.state === "running") {
    if (scan.phase === "sweep") return t("phase.sweep", { asked: scan.asked, total: scan.total });
    if (scan.phase === "objects") return t("phase.objects");
    return scan.phase === "details" ? t("phase.details") : t("phase.whoIs");
  }
  if (state.reading) return readingText(state.readProgress);
  return t("status.ready");
}

/** Where the background reading of a device stands, in words. */
export function readingText(progress) {
  if (progress?.phase === "list" && progress.total) return t("read.list", { read: progress.listed, total: progress.total });
  if (progress?.phase === "values") return t("read.values", { read: progress.read, total: progress.total });
  if (progress?.phase === "limits") return t("read.limits");
  return t("device.reading");
}

export function statusHtml(state) {
  const busy = state.scan.state === "running" || state.reading;
  const trunk = state.mstp?.trunk;
  const link = trunk ? t("mstp.status", { baud: trunk.baud }) : `${t("port")} ${state.settings.port.trim() || "47808"}`;
  const held = state.held.length ? `<button type="button" class="chip warn" data-area-go="log">${t("held", { n: state.held.length })}</button>` : "";
  const recording = state.recording?.running ? `<button type="button" class="chip accent" data-area-go="recording">● ${t("rec.on")}</button>` : "";
  return `<span class="state"><span class="dot ${busy ? "busy" : "on"}"></span>${h(activity(state))}</span>
    ${busy ? `<button type="button" class="quiet" data-act="stop-running">${t("stop")}</button>` : ""}
    <span class="cut">${h(link)}</span>
    <span class="chips">${held}${recording}${captureChipHtml(state)}</span>`;
}
