// Loading the plant view a device carries itself, once its points are read.

import { api } from "./api.js";
import { render, state, toast } from "./store.js";

/**
 * Reads the device's Structured View objects, unless they are already known; `fresh` asks again,
 * as *Read again* does. A device without them keeps an empty list, so the tree groups by names.
 */
export async function loadStructure(instance, fresh = false) {
  if (!state.points || state.device?.instance !== instance) return;
  if (!fresh && state.structure[instance]) return;
  try {
    const { views } = await api.structure(instance);
    state.structure[instance] = views;
  } catch (error) {
    state.structure[instance] = [];
    toast(error.message, true);
  }
  render();
}
