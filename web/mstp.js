// MS/TP in the search settings (docs/concept-mstp.md): the connection type, the serial port and
// speed, and what listening to the trunk heard. Listening sends nothing.

import { h } from "./html.js";
import { t } from "./i18n.js";

/** The speeds MS/TP trunks run at, as the server takes them. */
export const MSTP_SPEEDS = [9600, 19200, 38400, 76800, 115200];

/** BACnet/IP or MS/TP, first in the search settings; only where this bacnetio has MS/TP. */
export function linkChoiceHtml(state) {
  if (!state.mstpAvailable) return "";
  const choice = (link, label) => `<button type="button" id="link-${link}" data-act="link" data-link="${link}" aria-pressed="${state.link === link}">${t(label)}</button>`;
  return `<dt><span id="link-label">${t("link.title")}</span></dt><dd><span class="seg" role="group" aria-labelledby="link-label">${choice("ip", "link.ip")}${choice("mstp", "link.mstp")}</span></dd>`;
}

function portField(mstp) {
  if (mstp.ports === null) return `<span class="hint">${t("mstp.portsLoading")}</span>`;
  if (!mstp.ports.length) {
    return `<span class="hint">${t("mstp.noPort")}</span> <button type="button" class="quiet" data-act="mstp-ports">${t("mstp.lookAgain")}</button>`;
  }
  const options = mstp.ports.map((port) => {
    const label = port.description ? `${port.name} · ${port.description}` : port.name;
    return `<option value="${h(port.name)}"${port.name === mstp.port ? " selected" : ""}>${h(label)}</option>`;
  }).join("");
  return `<select id="mstp-port">${options}</select>`;
}

function speedField(mstp) {
  const options = [`<option value=""${mstp.baud === "" ? " selected" : ""}>${t("mstp.findSpeed")}</option>`,
    ...MSTP_SPEEDS.map((baud) => `<option value="${baud}"${String(baud) === mstp.baud ? " selected" : ""}>${baud}</option>`)].join("");
  return `<select id="mstp-baud">${options}</select>`;
}

/** The fields of the search settings with MS/TP chosen; bacnetio's address once a speed was heard. */
export function mstpFieldsHtml(state) {
  const mstp = state.mstp;
  const field = (id, label, input) => `<dt><label for="${id}">${label}</label></dt><dd>${input}</dd>`;
  const station = canJoin(mstp)
    ? `${field("mstp-station", t("mstp.ownAddress"), `<input id="mstp-station" class="mono" inputmode="numeric" value="${h(mstp.station)}">`)}
    <dd class="hint">${t("mstp.joinHint", { baud: mstp.heard.baud })}</dd>`
    : "";
  return `${field("mstp-port", t("mstp.port"), portField(mstp))}
    ${field("mstp-baud", t("mstp.speed"), speedField(mstp))}
    <dd class="hint">${t("mstp.listenHint")}</dd>
    ${station}`;
}

/** Joining needs a speed heard on this port: listening comes first (docs/concept-mstp.md). */
export const canJoin = (mstp) => Boolean(mstp.heard?.baud) && !mstp.listening;

/** The buttons of the dialog with MS/TP: listen, and once a speed was heard, join as the next step. */
export function mstpActsHtml(state) {
  const mstp = state.mstp;
  const busy = !mstp.port || mstp.listening;
  if (!canJoin(mstp)) return `<button type="button" data-act="mstp-listen"${busy ? " disabled" : ""}>${t("mstp.listen")}</button>`;
  return `<button type="button" class="quiet" data-act="mstp-listen"${busy ? " disabled" : ""}>${t("mstp.listenAgain")}</button>
    <button type="button" data-act="mstp-search">${t("mstp.join")}</button>`;
}

/** In the sidebar while bacnetio is on a trunk: where, and the way off it. */
export function trunkHtml(state) {
  const trunk = state.mstp.trunk;
  if (!trunk) return "";
  return `<div class="side-trunk"><span class="side-detail">${t("mstp.onTrunk", { station: trunk.station, baud: trunk.baud })}</span>
    <button type="button" class="quiet" data-act="mstp-leave">${t("mstp.leave")}</button></div>`;
}

/** What listening heard, in words: the speed, the stations, a free address; or why nothing came. */
export function heardHtml(state) {
  const { heard, error, listening } = state.mstp;
  if (listening) return `<div class="strip info mstp-heard" role="status">${t("mstp.listening")}</div>`;
  if (error) return `<div class="strip err mstp-heard" role="alert">${h(error)}</div>`;
  if (!heard) return "";
  if (heard.baud === null) {
    const damaged = heard.trials.filter((trial) => trial.damaged > 0).map((trial) => trial.baud);
    const text = damaged.length ? t("mstp.onlyDamaged", { speeds: damaged.join(", ") }) : t("mstp.nothingHeard");
    return `<div class="strip mstp-heard" role="status">${text}</div>`;
  }
  const stations = heard.stations.length ? heard.stations.join(", ") : "–";
  const free = heard.proposed_station === null ? t("mstp.noFree") : t("mstp.free", { station: heard.proposed_station });
  return `<div class="strip info mstp-heard" role="status"><dl class="facts">
    <dt>${t("mstp.speed")}</dt><dd class="mono">${heard.baud}</dd>
    <dt>${t("mstp.stations")}</dt><dd class="mono">${stations}</dd>
    <dt>${t("mstp.ownAddress")}</dt><dd>${free}</dd>
  </dl></div>`;
}
