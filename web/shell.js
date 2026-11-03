// The frame around every page: sidebar (network, areas, settings), page head and status bar.

import { h } from "./html.js";
import { t } from "./i18n.js";
import { deviceCount } from "./format.js";
import { resizeHandle } from "./panels.js";
import { siteProblems } from "./problems.js";
import { openSiteOf, siteTitle } from "./sites.js";

const ICON = (path) => `<svg class="nav-i" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${path}</svg>`;
export const AREA_ICONS = {
  workspace: ICON('<rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="5" rx="2"/><rect x="13" y="11" width="8" height="10" rx="2"/><rect x="3" y="14" width="8" height="7" rx="2"/>'),
  problems: ICON('<path d="M12 3l9.5 17h-19z"/><path d="M12 10v4.5M12 17.5v.5"/>'),
  watch: ICON('<path d="M2.5 12S6 5 12 5s9.5 7 9.5 7-3.5 7-9.5 7-9.5-7-9.5-7z"/><circle cx="12" cy="12" r="2.5"/>'),
  recording: ICON('<path d="M3 17l5-6 4 3 5-7 4 4"/><path d="M3 21h18"/>'),
  log: ICON('<path d="M14.5 4.5l5 5L9 20H4v-5z"/><path d="M12 7l5 5"/>'),
};
const SITES_ICON = ICON('<path d="M3 21h18"/><path d="M5 21V8l7-4 7 4v13"/><path d="M9 21v-5h6v5"/><path d="M9 10h.01M15 10h.01"/>');
const SETTINGS_ICON = ICON('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>');

function badge(state, area) {
  if (area === "workspace") return `<span class="n">${state.scan.devices?.length || ""}</span>`;
  if (area === "problems") return `<span class="n err">${siteProblems(state).length || ""}</span>`;
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
  if (scan.state === "done") return `<span class="state"><span class="dot on"></span>${deviceCount(scan.devices?.length ?? 0)}</span>`;
  return `<span class="state"><span class="dot"></span>${t("conn.ready")}</span>`;
}

/** Where the search looks, in one line: the subnet entered, or the adapter's. */
export function searchScope(state) {
  if (state.settings.subnet.trim()) return state.settings.subnet.trim();
  if (state.settings.bbmd.trim()) return `BBMD ${state.settings.bbmd.trim()}`;
  return state.adapter ? `${state.adapter.ip}/${state.adapter.prefix_len}` : "";
}

function networkMenu(state) {
  if (state.menu !== "network") return "";
  const value = (key) => h(state.settings[key]);
  return `<div class="menu dialog" role="dialog" aria-label="${t("net.title")}">
    <h3>${t("net.title")}</h3>
    <p class="hint">${t("vpn.hint")}</p>
    <label class="row"><span>${t("subnet")}</span><input id="subnet" class="mono" placeholder="192.168.10.0/24" value="${value("subnet")}"></label>
    <label class="row"><span>${t("bbmd")}</span><input id="bbmd" class="mono" placeholder="10.20.0.10" value="${value("bbmd")}"></label>
    <label class="row"><span>${t("port")}</span><input id="port" class="mono" inputmode="numeric" value="${value("port")}"></label>
    <label class="row"><span>${t("range.from")}</span><input id="id-from" class="mono" inputmode="numeric" placeholder="0" value="${value("idFrom")}"></label>
    <label class="row"><span>${t("range.to")}</span><input id="id-to" class="mono" inputmode="numeric" placeholder="4194302" value="${value("idTo")}"></label>
    <p class="hint">${t("range.hint")}</p>
    <div class="row end"><button type="button" class="quiet" data-act="close-menu">${t("cancel")}</button><button type="button" data-act="find">${t("searchWithSettings")}</button></div>
  </div>`;
}

/** The open site was searched where the next search would look: the line below the adapter would
 *  only repeat its network. The server names an adapter's own subnet by the adapter's address. */
function searchesOpenSite(state) {
  const site = openSiteOf(state);
  const scope = searchScope(state);
  return Boolean(site) && (scope === site.name || scope.startsWith(`${site.name}/`));
}

/** The site shown, named as the technician named it; a click leads to all sites. */
function siteLink(state) {
  const site = openSiteOf(state);
  const current = state.area === "sites" ? ' aria-current="page"' : "";
  const name = site ? `<b class="site-now">${h(siteTitle(site))}</b>` : "";
  const network = site?.title ? `<span class="site-net mono">${h(site.name)}</span>` : "";
  return `<a class="site-link" href="#/sites" data-area="sites"${current} title="${h(site ? `${siteTitle(site)} · ${site.name}` : t("sites.title"))}">${SITES_ICON}<span><span class="side-caption">${t("sites.title")}</span>${name}${network}</span></a>`;
}

export function sideHtml(state) {
  const options = state.adapters.map((adapter, index) => {
    const label = `${adapter.name} · ${adapter.ip}`;
    return `<option value="${index}"${adapter === state.adapter ? " selected" : ""}>${h(label)}</option>`;
  }).join("");
  const adapterField = state.adapters.length
    ? `<select id="adapter" aria-label="${t("adapter.title")}" title="${h(state.adapter?.description ?? state.adapter?.name ?? "")}">${options}</select>`
    : `<span class="hint">${t("start.noAdapter")}</span>`;
  // the keys 1 to 6 choose the areas in this order, settings last (app.js)
  const listed = ["workspace", "problems", "watch", "recording", "log"];
  const areas = listed.map((area, index) => tab(state, area, index + 1)).join("");
  const settingsCurrent = state.area === "settings" ? ' aria-current="page"' : "";
  return `<a class="brand" href="#/workspace"><img src="icon.svg" alt="">bacnetio</a>
    ${siteLink(state)}
    <div class="net">
      ${adapterField}
      <div class="conn menu-host">${connection(state)}
        <button type="button" class="quiet" data-act="network" aria-expanded="${state.menu === "network"}">${t("change")}</button>
        ${networkMenu(state)}</div>
      ${searchesOpenSite(state) ? "" : `<div class="where mono">${h(searchScope(state))}</div>`}
    </div>
    <label class="site-search"><input id="site-search" type="search" placeholder="${t("search.site")}" aria-keyshortcuts="/" value="${h(state.siteSearch)}" autocomplete="off"><kbd aria-hidden="true">/</kbd></label>
    <nav class="tabs" aria-label="${t("areas")}">${areas}</nav>
    <a class="quiet settings-link" href="#/settings" data-area="settings"${settingsCurrent} title="${t("tab.settings")} (${listed.length + 1})">${SETTINGS_ICON}<span class="side-label">${t("tab.settings")}</span></a>
    ${resizeHandle("side")}`;
}

/** Title, one sentence and the one next step: every page says where you are. */
export function nextbarHtml(view) {
  const next = view.next ? `<button type="button" id="next" data-act="${view.next.act}"${view.next.data ?? ""}>${h(view.next.label)}</button>` : "";
  return `<div class="pagehead"><h1>${h(view.title)}</h1><p class="lead">${view.lead ?? ""}</p></div>${next}`;
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
  const adapter = state.adapter ? `${state.adapter.name} ${state.adapter.ip}/${state.adapter.prefix_len}` : "";
  const port = state.settings.port.trim() || "47808";
  const held = state.held.length ? `<button type="button" class="chip warn" data-area-go="log">${t("held", { n: state.held.length })}</button>` : "";
  const recording = state.recording?.running ? `<button type="button" class="chip accent" data-area-go="recording">● ${t("rec.on")}</button>` : "";
  return `<span class="state"><span class="dot ${busy ? "busy" : "on"}"></span>${h(activity(state))}</span>
    ${busy ? `<button type="button" class="quiet" data-act="stop-running">${t("stop")}</button>` : ""}
    <span class="cut">${h(adapter)} · ${t("port")} ${h(port)}</span>
    <span class="chips">${held}${recording}</span>`;
}
