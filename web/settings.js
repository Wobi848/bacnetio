// Settings: language and theme. Both follow the system until chosen here; each choice applies at once.

import { h } from "./html.js";
import { LANGUAGES, languageChoice, t } from "./i18n.js";

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
  return `<h3>${t(`set.background.${theme}`)}</h3><span class="choices">${buttons}</span>`;
}

export function updateCheckOn() {
  try {
    return localStorage.getItem(UPDATE_CHOICE_KEY) !== "off";
  } catch {
    return true;
  }
}

function statusText(update) {
  if (!updateCheckOn()) return t("update.offHint");
  if (!update || update.checking) return t("update.checking");
  if (update.error) return `${t("update.failed")} <span class="mono">${h(update.error)}</span>`;
  if (update.newer) return `${t("update.newer", { version: h(update.version) })} <a href="${h(update.url)}" target="_blank" rel="noopener">${t("update.page")}</a>`;
  return update.version ? t("update.current") : t("update.none");
}

/** The settings card: this version, what the last check found, and the switch. */
export function updateCardHtml(state) {
  const on = updateCheckOn();
  const look = on ? `<button type="button" data-act="update-now"${state.update?.checking ? " disabled" : ""}>${t("update.now")}</button>` : "";
  return `<section class="card"><h2>${t("update.title")}</h2>
    <p>bacnetio ${h(state.version ?? "")}</p>
    <p class="hint update-status">${statusText(state.update)}</p>
    <h3>${t("update.check")}</h3>
    <span class="seg"><button type="button" data-act="update-check" data-on="true" aria-pressed="${on}">${t("update.on")}</button><button type="button" data-act="update-check" data-on="false" aria-pressed="${!on}">${t("update.off")}</button></span>
    ${look}
    <p class="hint">${t("update.privacy")}</p></section>`;
}

export function viewSettings(state) {
  const languages = [["auto", t("set.auto")], ...LANGUAGES].map(([code, name]) =>
    `<button type="button" data-act="language" data-code="${code}" aria-pressed="${languageChoice() === code}">${h(name)}</button>`).join("");
  const themes = THEMES.map((theme) =>
    `<button type="button" data-act="theme" data-theme="${theme}" aria-pressed="${themeChoice() === theme}">${t(`set.${theme}`)}</button>`).join("");
  const accents = ACCENTS.map((accent) =>
    `<button type="button" data-act="accent" data-accent="${accent}" aria-pressed="${accentChoice() === accent}"><span class="swatch accent-${accent}"></span>${t(`set.accent.${accent}`)}</button>`).join("");
  const keys = [["1 – 5", t("keys.areas")], ["6", t("tab.settings")], ["/", t("keys.search")], ["f", t("keys.filter")], ["↑ ↓", t("keys.move")], ["Home End ⇞ ⇟", t("keys.jump")], ["Enter", t("keys.open")], ["Esc", t("keys.close")]]
    .map(([key, what]) => `<dt><kbd>${key}</kbd></dt><dd>${what}</dd>`).join("");
  return {
    title: t("tab.settings"),
    lead: t("set.lead"),
    next: null,
    html: `<div class="page"><div class="cards">
      <section class="card"><h2>${t("language")}</h2><span class="seg">${languages}</span><p class="hint">${t("set.languageHint")}</p></section>
      <section class="card"><h2>${t("set.theme")}</h2><span class="seg">${themes}</span>
        <h2>${t("set.accent")}</h2><span class="choices">${accents}</span>
        <h2>${t("set.background")}</h2>${backgroundsHtml("light")}${backgroundsHtml("dark")}</section>
      ${updateCardHtml(state)}
      <section class="card"><h2>${t("set.keys")}</h2><dl class="facts">${keys}</dl></section>
    </div></div>`,
  };
}
