// Object names after the example of the KBOB guide "BACnet Anwendung" 2021, 3.2.2, built on the
// BBL directive on labelling building services: NNNN_AAX_ANN_ANNN_AN, e.g. 3022_HZ1_L13_M100_S0
// (ORT: economic unit, object and level; BTA; APPARAT; FUNKTION). The guide calls it an example:
// the owner sets the binding scheme, so this check is switched on by the user.

/** BTA kinds (system types); I is none. */
const SYSTEM_KINDS = "ABCDEFGHJKLMNOPQRSTUVWXYZ";
/** Device kinds of the process and of switchgear assemblies. */
const DEVICE_KINDS = "BDEFGHIKMPQRSTUXYZ";
/** FUNKTION, with the BACnet additions of the guide's table 2 (T, U, K, R, N). */
const FUNCTION_KINDS = "DZSMYHWTUKRN";
// own names: the build puts every file of the plugin into one scope (plugins/build.mjs)
const DEVICE_OBJECT = 8;
const CLASS_OBJECT = 15;

/** The blocks of a name, what is left over, and what does not fit. */
export function splitName(name) {
  const parts = {};
  const rest = [];
  for (const block of String(name ?? "").trim().toUpperCase().split(/[_\s]+/).filter(Boolean)) {
    if (/^\d{4}$/.test(block) && !parts.unit) parts.unit = block;
    else if (/^[A-Z]{2}[0-9A-Z]$/.test(block) && !parts.object) parts.object = block;
    else if (/^[A-Z]\d{2}$/.test(block) && !parts.system) parts.system = block;
    else if (/^[A-Z]\d{3}$/.test(block) && !parts.device) parts.device = block;
    else if (/^[A-Z]\d$/.test(block) && !parts.function) parts.function = block;
    else rest.push(block);
  }
  const problems = [];
  if (parts.system && !SYSTEM_KINDS.includes(parts.system[0])) problems.push(`BTA ${parts.system[0]}`);
  if (parts.system && /^[QR]00$/.test(parts.system)) problems.push(`BTA ${parts.system}`);
  if (parts.device && !DEVICE_KINDS.includes(parts.device[0])) problems.push(`APPARAT ${parts.device[0]}`);
  if (parts.function && !FUNCTION_KINDS.includes(parts.function[0])) problems.push(`FUNKTION ${parts.function[0]}`);
  return { parts, rest, problems };
}

/**
 * Whether an object's name follows the pattern: object, BTA and, for a device its APPARAT, for any
 * other object its FUNKTION, nothing left over. Notification classes are named by number, not so.
 */
export function followsAks(type, name) {
  if (type === CLASS_OBJECT) return true;
  const { parts, rest, problems } = splitName(name);
  const complete = parts.object && parts.system && (type === DEVICE_OBJECT ? parts.device : parts.function);
  return Boolean(complete) && !rest.length && !problems.length;
}
