// BACnet/SC in the search settings (docs/concept-sc.md): the hub, the site's three certificate files
// with what was found in each in words, and the SC network bacnetio is on in the sidebar.

import { h } from "./html.js";
import { currentLocale, t } from "./i18n.js";
import { releaseLabel } from "./log.js";

const LINES = ["ca", "own", "key"];
const GOOD = new Set(["valid", "matches"]);
/** What still works but wants attention: orange, as a certificate about to run out. */
const ATTENTION = new Set(["expires_soon", "chosen", "needs_password"]);
/** A file still waits for its password. */
const WAITING = new Set(["needs_password", "wrong_password"]);

const dateText = (seconds) => new Date(seconds * 1000).toLocaleDateString(currentLocale());

/** The hub's host name, as the sidebar and the status say where bacnetio is. */
export function hubHost(hub) {
  try {
    return new URL(hub).host;
  } catch {
    return hub;
  }
}

function stateChip(line) {
  const kind = GOOD.has(line.state) ? "ok" : ATTENTION.has(line.state) ? "warn" : line.state === "missing" ? "" : "err";
  const text = t(`sc.state.${line.state}`, {
    from: line.from ? dateText(line.from) : "",
    until: line.until ? dateText(line.until) : "",
  });
  return `<span class="chip ${kind}">${h(text)}</span>`;
}

function lineHtml(sc, name) {
  const line = sc.view?.certificates[name] ?? { state: "missing", file: null };
  const file = line.file ? `<span class="mono cut sc-file" title="${h(line.file)}">${h(line.file)}</span>` : "";
  const password = WAITING.has(line.state)
    ? `<dd class="sc-password"><input id="sc-password-${name}" type="password" autocomplete="off" aria-label="${t("sc.password", { file: line.file })}" placeholder="${t("sc.passwordPlaceholder")}" value="${h(sc.passwords[name] ?? "")}">
      <button type="button" data-act="sc-open" data-line="${name}">${t("sc.open")}</button></dd>`
    : "";
  return `<dt>${t(`sc.line.${name}`)}</dt>
    <dd class="sc-line">${file}${stateChip(line)}<label class="button">${t("sc.choose")}<input id="sc-file-${name}" data-line="${name}" type="file" hidden></label></dd>
    ${password}`;
}

/** The fields of the search settings with BACnet/SC chosen. */
export function scFieldsHtml(state) {
  const sc = state.sc;
  const certificates = sc.view?.certificates;
  const remember = certificates?.key_password
    ? `<dd class="sc-remember"><label><input id="sc-remember" type="checkbox"${sc.rememberKey ? " checked" : ""}> ${t("sc.rememberKey")}</label></dd>
      <dd class="hint">${t("sc.rememberHint")}</dd>`
    : "";
  return `<dt><label for="sc-hub">${t("sc.hub")}</label></dt>
    <dd><input id="sc-hub" class="mono" placeholder="wss://hub.example:443/" value="${h(sc.hub)}"></dd>
    <dd class="hint">${t("sc.hubHint")}</dd>
    ${LINES.map((name) => lineHtml(sc, name)).join("")}
    <dd class="hint">${t("sc.filesHint")}</dd>
    ${remember}`;
}

/** Connecting needs a hub and the three files that belong together. */
export const canConnect = (sc) => Boolean(sc.view?.certificates.ready) && sc.hub.trim().startsWith("wss://");

/** The button of the dialog with BACnet/SC. */
export function scActsHtml(state) {
  return `<button type="button" data-act="sc-search"${canConnect(state.sc) ? "" : " disabled"}>${t("sc.connect")}</button>`;
}

/** In the sidebar while bacnetio is on an SC network: the hub, and the way off it. */
export function scNetworkHtml(state) {
  const hub = state.sc.view?.hub;
  if (!hub) return "";
  // what bacnetio holds stays in the plant after leaving, as after closing bacnetio
  const count = state.held.length;
  if (state.sc.leaving && count) {
    return `<div class="side-trunk"><div class="strip" role="alert">${count === 1 ? t("sc.leaveHeldOne") : t("sc.leaveHeld", { n: count })}</div>
      <button type="button" data-act="release-all">${releaseLabel(count)}</button>
      <button type="button" class="quiet" data-act="sc-leave-anyway">${t("sc.leaveAnyway")}</button></div>`;
  }
  return `<div class="side-trunk"><span class="side-detail">${t("sc.onNetwork", { hub: h(hubHost(hub)) })}</span>
    <button type="button" class="quiet" data-act="sc-leave">${t("sc.leave")}</button></div>`;
}
