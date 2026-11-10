// The help: guides for the usual tasks, in the interface's language (docs/concept-help.md).

import { h } from "./html.js";
import { currentLanguage, t } from "./i18n.js";
import de from "./help/de.js";
import en from "./help/en.js";
import es from "./help/es.js";
import fr from "./help/fr.js";
import it from "./help/it.js";

/** The guides in every language of the interface; French, Italian and Spanish are drafts like their other texts. */
export const HELP_FILES = { en, de, fr, it, es };

/** The guides in the order of the list, each with its group and the place its first step is. */
export const GUIDES = [
  { id: "find", group: "start", target: "network" },
  { id: "points", group: "start" },
  { id: "vpn", group: "site", target: "network" },
  { id: "bbmd", group: "site", target: "network" },
  { id: "mstp", group: "site", target: "network" },
  { id: "sc", group: "site", target: "network" },
  { id: "write", group: "values", target: "log" },
  { id: "watch", group: "values", target: "watch" },
  { id: "record", group: "values", target: "recording" },
  { id: "reference", group: "values" },
  { id: "exports", group: "exports" },
  { id: "capture", group: "exports", target: "settings" },
  { id: "sites", group: "more", target: "sites" },
  { id: "plugins", group: "more", target: "settings" },
  { id: "keys", group: "more" },
];

/** Where a guide's button leads, and its words. */
export const TARGETS = {
  network: "help.goNetwork", log: "help.goLog", watch: "help.goWatch", recording: "help.goRecording",
  settings: "help.goSettings", sites: "help.goSites",
};

const words = () => HELP_FILES[currentLanguage()];

/** A guide's text with the interface's own names of buttons and fields, and other guides' titles. */
export function helpText(text) {
  return h(text)
    .replace(/\{btn:([\w.]+)\}/g, (_, key) => `<b>${h(t(key))}</b>`)
    .replace(/\{guide:(\w+)\}/g, (_, id) => `«${h(words()[id].title)}»`);
}

/** The keys and what they do, in the order of the keyboard's use. */
function keysHtml() {
  const keys = [["1 – 5", t("keys.areas")], ["6", t("tab.settings")], ["?", t("help.title")], ["/", t("keys.search")], ["f", t("keys.filter")], ["↑ ↓", t("keys.move")], ["Home End ⇞ ⇟", t("keys.jump")], ["Enter", t("keys.open")], ["Esc", t("keys.close")]];
  return `<dl class="facts">${keys.map(([key, what]) => `<dt><kbd>${key}</kbd></dt><dd>${what}</dd>`).join("")}</dl>`;
}

function listHtml(chosen) {
  const groups = [...new Set(GUIDES.map((guide) => guide.group))];
  return groups.map((group) => {
    const buttons = GUIDES.filter((guide) => guide.group === group).map((guide) =>
      `<button type="button" class="quiet" data-act="help-guide" data-guide="${guide.id}" aria-pressed="${guide.id === chosen}">${h(words()[guide.id].title)}</button>`).join("");
    return `<h3 class="cap">${t(`help.group.${group}`)}</h3>${buttons}`;
  }).join("");
}

function guideHtml(guide) {
  const text = words()[guide.id];
  const steps = text.steps ? `<ol class="help-steps">${text.steps.map((step) => `<li>${helpText(step)}</li>`).join("")}</ol>` : "";
  const causes = text.causes ? `<h3>${t("help.causes")}</h3><ul class="help-causes">${text.causes.map((cause) => `<li>${helpText(cause)}</li>`).join("")}</ul>` : "";
  const target = guide.target ? `<div class="acts"><button type="button" class="tonal" data-act="help-go" data-target="${guide.target}">${t(TARGETS[guide.target])}</button></div>` : "";
  const keys = guide.id === "keys" ? keysHtml() : "";
  return `<article class="card help-guide" id="help-guide" aria-labelledby="help-guide-title">
    <h2 id="help-guide-title">${h(text.title)}</h2>
    <p>${helpText(text.intro)}</p>${steps}${keys}${target}${causes}</article>`;
}

export function viewHelp(state) {
  const guide = GUIDES.find((one) => one.id === state.helpGuide) ?? GUIDES[0];
  return {
    title: t("help.title"),
    lead: t("help.lead"),
    next: null,
    html: `<div class="page"><div class="help">
      <nav class="card help-list" aria-label="${t("help.guides")}">${listHtml(guide.id)}</nav>
      ${guideHtml(guide)}
    </div></div>`,
  };
}
