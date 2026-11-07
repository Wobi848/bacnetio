// MS/TP in the search settings (docs/concept-mstp.md): choosing the connection, the serial
// ports, and listening to a trunk, which sends nothing.

import { api } from "./api.js";
import { runSearch } from "./device-actions.js";
import { render, state, toast } from "./store.js";

/** BACnet/IP or MS/TP; the serial ports are asked for the first time MS/TP is chosen. */
export function chooseLink(link) {
  state.link = link;
  render();
  if (link === "mstp" && state.mstp.ports === null) loadSerialPorts();
}

/** The serial ports, the chosen one kept while it is still there, else the first (USB first). */
export async function loadSerialPorts() {
  const mstp = state.mstp;
  mstp.ports = null;
  render();
  try {
    mstp.ports = await api.serialPorts();
    mstp.error = null;
  } catch (error) {
    mstp.ports = [];
    mstp.error = error.message;
  }
  if (!mstp.ports.some((port) => port.name === mstp.port)) mstp.port = mstp.ports[0]?.name ?? "";
  render();
}

/** Listens to the trunk on the chosen port, at the chosen speed or each in turn. */
export async function listenToTrunk() {
  const mstp = state.mstp;
  if (!mstp.port || mstp.listening) return;
  mstp.listening = true;
  mstp.heard = null;
  mstp.error = null;
  render();
  try {
    mstp.heard = await api.mstpListen(mstp.port, mstp.baud ? Number(mstp.baud) : undefined);
    mstp.station = mstp.heard.proposed_station === null ? "" : String(mstp.heard.proposed_station);
  } catch (error) {
    mstp.error = error.message;
  }
  mstp.listening = false;
  render();
}

/** Joins the trunk heard as the station in the field and searches it; the server checks again. */
export async function searchOnTrunk() {
  const mstp = state.mstp;
  const station = Number(mstp.station);
  if (!mstp.heard?.baud || mstp.station.trim() === "" || !Number.isInteger(station)) return;
  state.menu = null;
  const joined = await runSearch(() => api.mstpSearch(mstp.port, mstp.heard.baud, station));
  if (joined) await loadTrunk();
}

/** Searches the trunk bacnetio is on again, as it joined it. */
export function searchTrunkAgain() {
  const trunk = state.mstp.trunk;
  return runSearch(() => api.mstpSearch(trunk.port, trunk.baud, trunk.station));
}

/** Leaves the trunk; its port is free again for listening. */
export async function leaveTrunk() {
  try {
    await api.mstpLeave();
    state.mstp.trunk = null;
    state.log = await api.log();
  } catch (error) {
    toast(error.message, true);
  }
  render();
}

/** Whether bacnetio is on a trunk, also after a reload of the page: MS/TP is chosen then. */
export async function loadTrunk() {
  const trunk = await api.mstpTrunk();
  state.mstp.trunk = trunk;
  if (trunk) {
    state.link = "mstp";
    state.mstp.port = trunk.port;
  }
  render();
}
