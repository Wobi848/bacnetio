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
 * The FUNKTION each object type stands for: the object types of the building automation functions
 * (recommendation, 7.2) with the letters of the guide's table 2. A meter or measured value is an AV
 * there (Zähl-/Messwert), so an AV may carry Z or M besides W.
 */
const FUNCTIONS_OF_TYPE = {
  0: "M", 1: "Y", 2: "WZM", 3: "D", 4: "S", 5: "H", 6: "K", 9: "H", 12: "R", 13: "D", 14: "S",
  16: "H", 17: "U", 19: "H", 20: "T", 25: "T", 48: "W",
};

/** Whether an object's name follows the pattern: object, BTA and, for a device its APPARAT, for any
 * other object its FUNKTION, nothing left over. Notification classes are named by number, not so. */
export function followsAks(type, name) {
  return aksProblem(type, name) === null;
}

/**
 * What is wrong with an object's name: null, { pattern: true } when it does not follow the
 * pattern, or { letter, expected } when its FUNKTION does not fit its object type.
 */
export function aksProblem(type, name) {
  if (type === CLASS_OBJECT) return null;
  const { parts, rest, problems } = splitName(name);
  const complete = parts.object && parts.system && (type === DEVICE_OBJECT ? parts.device : parts.function);
  if (!complete || rest.length || problems.length) return { pattern: true };
  const expected = FUNCTIONS_OF_TYPE[type];
  const letter = parts.function?.[0];
  if (expected && letter && !expected.includes(letter)) return { letter, expected: [...expected].join("/") };
  return null;
}
