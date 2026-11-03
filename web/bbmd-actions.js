// Reading the tables of a BBMD for the chosen device, when the user asks for them.

import { api } from "./api.js";
import { render, state, toast } from "./store.js";

/** Reads BDT and FDT of the chosen device. Both are only read; nothing is written. */
export async function readBbmdTables() {
  const device = state.selectedDevice;
  if (device === null) return;
  state.bbmdReading = device;
  render();
  try {
    state.bbmd[device] = await api.bbmdTables(device);
  } catch (error) {
    toast(error.message, true);
  }
  state.bbmdReading = null;
  render();
}
