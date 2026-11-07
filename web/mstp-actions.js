// MS/TP in the search settings (docs/concept-mstp.md): choosing the connection, the serial
// ports, and listening to a trunk, which sends nothing.

import { api } from "./api.js";
import { render, state } from "./store.js";

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
  } catch (error) {
    mstp.error = error.message;
  }
  mstp.listening = false;
  render();
}
