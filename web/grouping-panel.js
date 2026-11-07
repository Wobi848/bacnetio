// The site's own grouping pattern, set above the point list with a preview on the open device's
// points (docs/concept-toolbox.md part 4).

import { compilePattern, splitByPattern } from "./grouping.js";
import { h } from "./html.js";
import { t } from "./i18n.js";
import { openSiteOf } from "./sites.js";

/** The longest pattern; the server stops there too. */
export const LONGEST_PATTERN = 200;
/** Points shown in the preview: enough to see the parts, few enough to stay above the list. */
const PREVIEW_POINTS = 6;

/** The pattern being typed as a regular expression, or the browser's reason why it is none. */
function tryPattern(text) {
  try {
    return { pattern: compilePattern(text) };
  } catch (error) {
    return { reason: error.message };
  }
}

function previewHtml(points, text) {
  if (!text.trim()) return `<p class="hint">${t("grouping.automatic")}</p>`;
  const { pattern, reason } = tryPattern(text);
  if (!pattern) return `<div class="strip err">${h(t("grouping.invalid", { reason }))}</div>`;
  const splits = points.map((point) => ({ point, split: splitByPattern(point.name, pattern) }));
  const fitting = splits.filter(({ split }) => split);
  const rows = fitting.slice(0, PREVIEW_POINTS).map(({ point, split }) =>
    `<li><span class="mono sub">${h(point.name)}</span> → <b>${h(split.part)}</b> · ${h(split.name)}</li>`).join("");
  const count = t("grouping.fit", { fit: fitting.length, n: points.length, other: points.length - fitting.length });
  return `<p class="hint">${count}</p>${rows ? `<ul class="grouping-preview">${rows}</ul>` : ""}`;
}

/** The panel while the pattern is edited; nothing otherwise. */
export function groupingPanelHtml(state, points) {
  if (typeof state.groupingEdit !== "string") return "";
  const site = openSiteOf(state);
  const text = state.groupingEdit;
  const valid = !text.trim() || Boolean(tryPattern(text).pattern);
  const automatic = site?.grouping ? `<button type="button" class="quiet" data-act="grouping-auto">${t("grouping.toAutomatic")}</button>` : "";
  return `<section class="grouping-panel" aria-label="${t("grouping.title")}">
    <h3>${t("grouping.title")}</h3>
    <p class="hint">${t("grouping.lead")}</p>
    <label class="grouping-field"><span>${t("grouping.pattern")}</span><input id="grouping-pattern" class="mono" maxlength="${LONGEST_PATTERN}" placeholder="^(\\w+_\\w+_\\d+)_" value="${h(text)}" autocomplete="off" spellcheck="false"></label>
    ${previewHtml(points, text)}
    <span class="acts"><button type="button" data-act="grouping-save"${valid ? "" : " disabled"}>${t("grouping.save")}</button>${automatic}<button type="button" class="quiet" data-act="grouping-cancel">${t("cancel")}</button></span>
  </section>`;
}

/** The menu entry; without a site to keep the pattern with, it says why it cannot be used. */
export function groupingMenuHtml(state) {
  const site = openSiteOf(state);
  const title = site ? t("grouping.hint") : t("grouping.noSite");
  return `<button type="button" class="quiet" data-act="grouping-edit" title="${title}"${site ? "" : " disabled"}>${t("grouping.open")}</button>`;
}
