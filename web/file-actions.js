// A device's File object saved on this computer: read whole from the device, never written back.

import { api } from "./api.js";
import { render, state, toast } from "./store.js";

/** Reads the chosen File object and hands it to the browser to save under the object's name. */
export async function saveFile() {
  const object = state.object;
  state.savingFile = true;
  render();
  try {
    const content = await api.file(state.device.instance, object.type, object.instance);
    const link = document.createElement("a");
    link.href = URL.createObjectURL(content);
    link.download = object.name || `file-${state.device.instance}-${object.instance}`;
    link.click();
    URL.revokeObjectURL(link.href);
  } catch (error) {
    toast(error.message, true);
  }
  state.savingFile = false;
  render();
}
