// Settings: language and theme. Both follow the system until chosen here; each choice applies at once.

import { captureCardHtml } from "./capture.js";
import { scKeyCardHtml } from "./sc.js";
import { h } from "./html.js";
import { LANGUAGES, languageChoice, t } from "./i18n.js";
import { pluginsCardHtml } from "./plugins.js";

const THEMES = ["auto", "light", "dark"];
const THEME_KEY = "bacnetio.theme";
/** Whether the page may ask for a new version (update.js); on unless switched off. */
export const UPDATE_CHOICE_KEY = "bacnetio.updateCheck";
/** Accent colours to choose from; states keep their own colours whatever is chosen. */
const ACCENTS = ["blue", "crimson", "violet", "teal"];
const ACCENT_KEY = "bacnetio.accent";
/** Backgrounds of each theme, the first as it was before there was a choice (theme.js). */
const BACKGROUNDS = { light: ["white", "grey", "warm"], dark: ["navy", "graphite", "black"] };
const BACKGROUND_KEY = { light: "bacnetio.background.light", dark: "bacnetio.background.dark" };

export function themeChoice() {
  try {
    return localStorage.getItem(THEME_KEY) || "auto";
  } catch {
    return "auto";
  }
}

export function setTheme(choice) {
  try {
    localStorage.setItem(THEME_KEY, choice);
  } catch {
    // the choice then lasts only until the page is reloaded
  }
  // applyTheme comes from theme.js, which runs before the first drawing
  globalThis.applyTheme();
}

export function accentChoice() {
  try {
    return localStorage.getItem(ACCENT_KEY) || "blue";
  } catch {
    return "blue";
  }
}

export function setAccent(choice) {
  try {
    localStorage.setItem(ACCENT_KEY, choice);
  } catch {
    // the choice then lasts only until the page is reloaded
  }
  globalThis.applyTheme();
}

export function backgroundChoice(theme) {
  try {
    const chosen = localStorage.getItem(BACKGROUND_KEY[theme]);
    return BACKGROUNDS[theme].includes(chosen) ? chosen : BACKGROUNDS[theme][0];
  } catch {
    return BACKGROUNDS[theme][0];
  }
}

export function setBackground(theme, choice) {
  try {
    localStorage.setItem(BACKGROUND_KEY[theme], choice);
  } catch {
    // the choice then lasts only until the page is reloaded
  }
  globalThis.applyTheme();
}

/** A row of backgrounds for the light or the dark theme, each with its own swatch. */
function backgroundsHtml(theme) {
  const buttons = BACKGROUNDS[theme].map((name) =>
    `<button type="button" data-act="background" data-scheme="${theme}" data-background="${name}" aria-pressed="${backgroundChoice(theme) === name}"><span class="swatch background-${name}"></span>${t(`set.background.${name}`)}</button>`).join("");
  return `<div class="choice-row"><span class="hint">${t("set.background")} · ${t(`set.background.${theme}`)}</span><span class="choices">${buttons}</span></div>`;
}

export function updateCheckOn() {
  try {
    return localStorage.getItem(UPDATE_CHOICE_KEY) !== "off";
  } catch {
    return true;
  }
}

/** What the last check found, in one sentence; a failure as a strip, its technical text on demand. */
function statusHtml(state) {
  const update = state.update;
  if (!updateCheckOn()) return `<p class="update-status">${t("update.offHint")}</p>`;
  if (!update || update.checking) return `<p class="update-status">${t("update.checking")}</p>`;
  if (update.error) {
    const details = state.updateDetails ? `<p class="mono update-details">${h(update.error)}</p>` : "";
    return `<div class="strip update-status">${t("update.failed")}
      <span class="acts"><button type="button" class="quiet" data-act="update-details" aria-expanded="${Boolean(state.updateDetails)}">${t(state.updateDetails ? "update.hideDetails" : "update.showDetails")}</button></span>${details}</div>`;
  }
  if (update.newer) return `<p class="update-status">${t("update.newer", { version: h(update.version) })}</p><span class="acts"><a class="quiet" href="${h(update.url)}" target="_blank" rel="noopener">${t("update.page")}</a></span>`;
  return `<p class="update-status">${update.version ? t("update.current") : t("update.none")}</p>`;
}

/** The settings card (Card): this version large, what the last check found, the switch and the tool. */
export function updateCardHtml(state) {
  const on = updateCheckOn();
  const look = on ? `<div class="acts"><button type="button" class="quiet" data-act="update-now"${state.update?.checking ? " disabled" : ""}>${t("update.now")}</button></div>` : "";
  return `<section class="card"><h2>${t("update.title")}</h2>
    <div class="figure">${h(state.version ?? "")}</div>
    ${statusHtml(state)}
    <span class="choices" role="group" aria-label="${t("update.check")}"><button type="button" data-act="update-check" data-on="true" aria-pressed="${on}">${t("update.checkOn")}</button><button type="button" data-act="update-check" data-on="false" aria-pressed="${!on}">${t("update.checkOff")}</button></span>
    <p class="hint">${t("update.privacy")}</p>
    ${look}</section>`;
}

export function viewSettings(state) {
  const languages = [["auto", t("set.auto")], ...LANGUAGES].map(([code, name]) =>
    `<button type="button" data-act="language" data-code="${code}" aria-pressed="${languageChoice() === code}">${h(name)}</button>`).join("");
  const themes = THEMES.map((theme) =>
    `<button type="button" data-act="theme" data-theme="${theme}" aria-pressed="${themeChoice() === theme}">${t(`set.${theme}`)}</button>`).join("");
  const accents = ACCENTS.map((accent) =>
    `<button type="button" data-act="accent" data-accent="${accent}" aria-pressed="${accentChoice() === accent}"><span class="swatch accent-${accent}"></span>${t(`set.accent.${accent}`)}</button>`).join("");
  return {
    title: t("tab.settings"),
    lead: t("set.lead"),
    next: null,
    // no next step on this page, so nothing is filled: every choice is light (PageSettings)
    html: `<div class="page"><div class="cards">
      <section class="card"><h2>${t("language")}</h2><span class="choices">${languages}</span><p class="hint">${t("set.languageHint")}</p></section>
      <section class="card"><h2>${t("set.look")}</h2>
        <div class="choice-row"><span class="hint">${t("set.theme")}</span><span class="seg">${themes}</span></div>
        <div class="choice-row"><span class="hint">${t("set.accent")}</span><span class="choices">${accents}</span></div>
        ${backgroundsHtml("light")}${backgroundsHtml("dark")}</section>
      ${updateCardHtml(state)}
      ${pluginsCardHtml(state)}
      ${captureCardHtml(state)}
      ${scKeyCardHtml(state)}
      <section class="card"><h2>${t("set.keys")}</h2><p class="hint">${t("help.keysMoved")}</p>
        <div class="acts"><button type="button" class="quiet" data-act="help-guide" data-guide="keys">${t("help.showKeys")}</button></div></section>
    </div></div>`,
  };
}
