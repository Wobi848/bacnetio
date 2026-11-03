// The tables of a BBMD in the device details: which BBMDs it distributes broadcasts to (BDT) and
// which devices are registered with it (FDT). Read only; bacnetio never changes such a table.

import { h } from "./html.js";
import { noteText, t } from "./i18n.js";

/** All ones: the broadcast goes to that BBMD itself instead of to its subnet (two-hop). */
const TO_THE_BBMD = "255.255.255.255";

function peerRows(peers) {
  if (!peers.length) return `<p class="hint">${t("bbmd.bdtEmpty")}</p>`;
  const rows = peers.map((peer) => {
    const where = peer.mask === TO_THE_BBMD ? t("bbmd.twoHop") : t("bbmd.subnet", { mask: h(peer.mask) });
    return `<dt class="mono">${h(peer.address)}</dt><dd>${where}${peer.own ? ` · ${t("bbmd.own")}` : ""}</dd>`;
  });
  return `<dl class="facts">${rows.join("")}</dl>`;
}

function foreignRows(entries) {
  if (!entries.length) return `<p class="hint">${t("bbmd.fdtEmpty")}</p>`;
  const rows = entries.map((entry) =>
    `<dt class="mono">${h(entry.address)}</dt><dd>${t("bbmd.ttl", { n: entry.ttl_s })} · ${t("bbmd.left", { n: entry.seconds_left })}</dd>`);
  return `<dl class="facts">${rows.join("")}</dl>`;
}

const table = (title, rows, note) =>
  `<h4>${title}</h4>${rows ?? `<p class="hint">${h(noteText(note))}</p>`}`;

/** The section, with the button to read the tables where they are not read yet. */
export function bbmdHtml(state) {
  const device = state.selectedDevice;
  const heading = `<h3>${t("bbmd.title")}</h3>`;
  if (state.bbmdReading === device) return `${heading}<p class="hint">${t("bbmd.reading")}</p>`;
  const tables = state.bbmd[device];
  if (!tables) {
    return `${heading}<p class="hint">${t("bbmd.hint")}</p>
      <div class="actions"><button type="button" class="quiet" data-act="bbmd-read">${t("bbmd.read")}</button></div>`;
  }
  const lead = tables.is_bbmd ? `<p class="hint">${t("bbmd.isBbmd")}</p>` : "";
  return `${heading}${lead}
    ${table(t("bbmd.bdt"), tables.bdt && peerRows(tables.bdt), tables.bdt_note)}
    ${table(t("bbmd.fdt"), tables.fdt && foreignRows(tables.fdt), tables.fdt_note)}
    <div class="actions"><button type="button" class="quiet" data-act="bbmd-read">${t("bbmd.readAgain")}</button></div>`;
}
