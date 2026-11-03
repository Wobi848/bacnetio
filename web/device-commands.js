// Commands to the chosen device: a new ID, restart and communication, acknowledging an alarm, the
// question who has a name, and which priority wins on its points.

import { api } from "./api.js";
import { find, localClock } from "./device-actions.js";
import { DEVICE_TYPE } from "./format.js";
import { errorText, t } from "./i18n.js";
import { refreshHeld, render, state, toast } from "./store.js";

/** BACnet's number for the object-name property. */
const OBJECT_NAME = 77;
/** The longest a device's communication can be stopped by time: the service counts in 16 bits. */
const MOST_STOP_MINUTES = 65535;

/** The device answers under the new ID only, so the search runs again at once. */
export async function changeInstance() {
  const device = state.selectedDevice;
  const instance = Number(state.newInstance);
  if (!state.newInstance.trim() || !Number.isInteger(instance)) {
    toast(t("instance.invalid"), true);
    return;
  }
  try {
    await api.changeInstance(device, instance);
    state.asking = false;
    state.newInstance = "";
    toast(t("instance.done", { from: device, to: instance }));
    await refreshHeld();
    await find();
  } catch (error) {
    toast(errorText(error.message), true);
  }
}

/** The device object's name: the list, the tree and the details show the new one at once. */
export async function renameDevice() {
  const device = state.selectedDevice;
  const name = state.newName.trim();
  if (!name) {
    toast(t("name.empty"), true);
    return;
  }
  try {
    await api.writeProperty(device, DEVICE_TYPE, device, { property_id: OBJECT_NAME, data_type: "text", value: name });
    state.asking = false;
    state.newName = "";
    for (const found of [...(state.scan.devices ?? []), state.device]) {
      if (found?.instance === device) found.name = name;
    }
    toast(t("name.done", { n: device, name }));
  } catch (error) {
    toast(errorText(error.message), true);
  }
  render();
}

/** Restart or stop or resume the chosen device, as confirmed. */
export async function runCommand(kind) {
  const device = state.selectedDevice;
  const password = state.commandPassword || null;
  const typed = state.commandMinutes.trim();
  const minutes = typed ? Number(typed) : null;
  // a slip here must not turn into stopping a device for good
  if (kind === "stop" && typed && !(Number.isInteger(minutes) && minutes >= 1 && minutes <= MOST_STOP_MINUTES)) {
    toast(t("cmd.minutesInvalid"), true);
    return;
  }
  try {
    if (kind === "warm" || kind === "cold") await api.restart(device, kind, password);
    else await api.communication(device, kind === "resume", kind === "stop" ? minutes : null, password);
    toast(t(`cmd.${kind}Done`));
    state.asking = false;
    state.commandMinutes = "";
    state.log = await api.log();
  } catch (error) {
    toast(errorText(error.message), true);
  }
  // kept no longer than this one try
  state.commandPassword = "";
  render();
}

/** Acknowledging tells the plant someone has seen the alarm, so the click asks first. */
export function askAcknowledge(type, instance) {
  state.asking = `ack:${type}:${instance}`;
  render();
}

export async function acknowledgeAlarm(type, instance) {
  const device = state.selectedDevice;
  state.asking = false;
  try {
    await api.acknowledge(device, { type, instance, ...localClock() });
    toast(t("alarms.acked"));
    state.deviceAlarms = await api.alarms(device);
    state.log = await api.log();
  } catch (error) {
    toast(errorText(error.message), true);
  }
  render();
}

/** Asks the whole network who has an object of this name: also devices nobody has read yet. */
export async function askWhoHas() {
  state.whoHas = { asking: true, found: [] };
  render();
  try {
    state.whoHas = { found: await api.whoHas(state.siteSearch) };
  } catch (error) {
    state.whoHas = null;
    toast(error.message, true);
  }
  render();
}

/** Which priority wins on each object of the open device, read when asked: one request per object. */
export async function readWinners() {
  state.readingWinners = true;
  render();
  try {
    const winners = await api.priorities(state.device.instance);
    state.winners = Object.fromEntries(winners.map((item) => [`${item.type}:${item.instance}`, item.winner]));
  } catch (error) {
    toast(error.message, true);
  }
  state.readingWinners = false;
  render();
}
