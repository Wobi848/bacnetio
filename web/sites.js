// Sites: every search keeps one, named by its network until the technician gives it a name.

import { deviceCount } from "./format.js";
import { h } from "./html.js";
import { currentLocale, t } from "./i18n.js";

/** The longest name a site can be given; the server stops there too. */
const LONGEST_TITLE = 80;

/** What a site is called: the name given, else the network it was searched in. */
export const siteTitle = (site) => site.title || site.name;

/** The site shown now, if it is one of those kept. */
export const openSiteOf = (state) => state.sites.find((site) => site.open) ?? null;

function actionsHtml(state, site, index) {
  if (state.deletingSite === index) {
    return `<button type="button" data-act="site-delete">${t("sites.deleteYes")}</button>
      <button type="button" class="quiet" data-act="site-keep">${t("cancel")}</button>`;
  }
  const open = site.open ? "" : `<button type="button" data-act="site-open" data-index="${index}">${t("sites.open")}</button>`;
  return `${open}
    <button type="button" class="quiet" data-act="site-rename" data-index="${index}">${t("sites.rename")}</button>
    <a class="quiet" href="/api/site.json?name=${encodeURIComponent(site.name)}" download>${t("sites.file")}</a>
    <button type="button" class="quiet" data-act="site-ask-delete" data-index="${index}">${t("sites.delete")}</button>`;
}

function nameHtml(state, site, index) {
  if (state.renamingSite === index) {
    return `<form class="site-rename" data-index="${index}">
      <input id="site-title" aria-label="${t("sites.name")}" placeholder="${h(site.name)}" maxlength="${LONGEST_TITLE}" value="${h(state.siteTitle)}" autocomplete="off">
      <button type="submit">${t("sites.save")}</button>
      <button type="button" class="quiet" data-act="site-rename-cancel">${t("cancel")}</button>
    </form>`;
  }
  const network = site.title ? `<span class="mono sub">${h(site.name)}</span>` : "";
  const shown = site.open ? `<span class="chip accent">${t("sites.shown")}</span>` : "";
  return `<h3>${h(siteTitle(site))} ${shown}</h3>${network}`;
}

function siteHtml(state, site, index) {
  const facts = t("sites.facts", {
    devices: deviceCount(site.devices),
    read: site.read,
    when: new Date(site.saved_ms).toLocaleString(currentLocale()),
  });
  return `<li class="site${site.open ? " open" : ""}">
    <div class="site-name">${nameHtml(state, site, index)}</div>
    <p class="hint">${h(facts)}</p>
    <div class="acts">${actionsHtml(state, site, index)}</div>
  </li>`;
}

export function viewSites(state) {
  const list = state.sites.length
    ? `<ul class="sites">${state.sites.map((site, index) => siteHtml(state, site, index)).join("")}</ul>`
    : `<p class="hint">${t("sites.none")}</p>`;
  return {
    title: t("sites.title"),
    lead: t("sites.lead"),
    next: { act: "site-new", label: t("sites.new") },
    html: `<div class="page">
      ${list}
      <div class="acts"><label class="button quiet">${t("sites.import")}<input id="site-file" type="file" accept=".json,application/json" hidden></label></div>
    </div>`,
  };
}
