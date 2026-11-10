// The capture of bacnetio's own traffic for Wireshark (docs/concept-toolbox.md part 3): its card in
// the settings and its chip in the status bar while it runs.

import { t } from "./i18n.js";

const count = (capture) => (capture.frames === 1 ? t("capture.one") : t("capture.many", { n: capture.frames.toLocaleString() }));

export function captureCardHtml(state) {
  const capture = state.capture ?? { running: false, frames: 0, overflowed: 0 };
  const toggle = capture.running
    ? `<button type="button" class="quiet" data-act="capture-stop">${t("capture.stop")}</button>`
    : `<button type="button" class="quiet" data-act="capture-start">${t("capture.start")}</button>`;
  const kept = capture.frames
    ? `<a class="quiet" href="/api/capture.pcapng" download>${t("capture.save")}</a>
       <button type="button" class="quiet" data-act="capture-clear">${t("capture.clear")}</button>`
    : "";
  const overflowed = capture.overflowed ? `<p class="hint warn">${t("capture.overflowed", { n: capture.overflowed.toLocaleString() })}</p>` : "";
  return `<section class="card capture"><h2>${t("capture.title")}</h2>
    <p class="hint">${t("capture.lead")}</p>
    <div class="figure">${count(capture)}</div>
    <p class="update-status">${t(capture.running ? "capture.running" : "capture.stopped")}</p>
    ${overflowed}
    ${state.sc?.view?.hub ? `<p class="hint warn">${t("capture.notOnSc")}</p>` : ""}
    <div class="acts">${toggle}${kept}</div>
    <p class="hint">${t("capture.what")}</p></section>`;
}

/** In the status bar while a capture runs; a click leads to its card. */
export function captureChipHtml(state) {
  if (!state.capture?.running) return "";
  return `<button type="button" class="chip accent" data-area-go="settings">● ${t("capture.chip", { what: count(state.capture) })}</button>`;
}
