// The interface's language: one file per language in lang/ with the same keys, English as the
// fallback (docs/concept-languages.md). The system's language is taken unless one was chosen.

import de from "./lang/de.js";
import en from "./lang/en.js";
import es from "./lang/es.js";
import fr from "./lang/fr.js";
import it from "./lang/it.js";

export const LANGUAGE_FILES = { en, de, fr, it, es };
/** Languages in the menu. The others wait for a review by someone from the trade; reviewers open
 * them with ?lang=fr, ?lang=it or ?lang=es. */
export const LANGUAGES = [["en", "English"], ["de", "Deutsch"]];
const STORAGE_KEY = "bacnetio.language";

/** Server errors by pattern; each language names the key's text. */
const ERROR_PATTERNS = [
  [/^search for devices first$/, "searchFirst"],
  [/^a search is already running$/, "searching"],
  // opening a serial port without the rights for it (Linux: the group dialout)
  [/^(.+) is not a serial port of this computer$/, "notSerialPort"],
  [/^(\d+) baud is no MS\/TP speed: .*$/, "noMstpSpeed"],
  [/^listen to the trunk first$/, "mstpListenFirst"],
  [/^station (\d+) answered while listening: another device uses it$/, "mstpStationUsed"],
  [/^the trunk was heard at (\d+) baud, not (\d+): listen again at \d+ first$/, "mstpWrongSpeed"],
  [/^no master took bacnetio into the token ring as station (\d+) within (\d+) s: .*$/, "mstpNotInRing"],
  [/^bacnetio is on this trunk: leave it before listening again$/, "mstpOnTrunk"],
  [/^only a device on BACnet\/IP has BBMD tables$/, "mstpNoBbmd"],
  // a serial port that cannot be opened, with the system's reason after the colon
  [/^cannot open ([^\s:]+)(?:: (.*))?$/, "mstpCannotOpen"],
  [/^cannot join the MS\/TP trunk on ([^\s:]+)(?:: (.*))?$/, "mstpCannotJoin"],
  [/^device (\d+) sits on an MS\/TP trunk as station (\d+): listen to the trunk and join it first$/, "mstpHeldOnTrunk"],
  [/^device (\d+) is on BACnet\/IP: leave the trunk and search on BACnet\/IP first$/, "mstpHeldOnIp"],
  [/^no permission to open (\S+): add yourself to the group dialout .*$/, "mstpPermission"],
  [/^read the device's points first$/, "readFirst"],
  [/^add a point to the recording first$/, "recordFirst"],
  [/^this point is not in the recording$/, "notRecorded"],
  // the most frequent refusals in words; the code stays for whoever looks it up
  [/^the device refused: BACnet error: \S+ \/ write-access-denied$/, "refusedWriteAccess"],
  [/^the device refused: BACnet error: \S+ \/ password-failure$/, "refusedPassword"],
  [/^the device refused: BACnet error: \S+ \/ value-out-of-range$/, "refusedRange"],
  [/^the device refused: BACnet error: \S+ \/ invalid-data-type$/, "refusedDataType"],
  [/^the device refused: BACnet error: \S+ \/ unknown-property$/, "refusedUnknownProperty"],
  [/^the device refused: BACnet error: \S+ \/ unknown-object$/, "refusedUnknownObject"],
  [/^the device refused: BACnet error: \S+ \/ service-request-denied$/, "refusedService"],
  [/^the device no longer takes this stop/, "withdrawnStop"],
  [/^the device refused: (.*)$/, "refused"],
  [/^"(.*)" is not a number$/, "notNumber"],
  [/^"(.*)" is not active or inactive$/, "notBinary"],
  [/^"(.*)" is not a whole number$/, "notWhole"],
  [/^"(.*)" is not a positive whole number$/, "notPositiveWhole"],
  [/^"(.*)" is not a lighting state \(0 off, 1 on, \.\.\.\)$/, "notLighting"],
  [/^"(.*)" is not a state number \(1, 2, \.\.\.\)$/, "notState"],
  [/^objects of this type cannot be written yet$/, "notWritable"],
  [/^the BBMD at (\S+) refused the registration.*$/, "bbmdRefused"],
  [/^the BBMD at (\S+) does not answer.*$/, "bbmdSilent"],
  [/^the BBMD at (\S+) did not answer the registration within (\d+) s$/, "bbmdLate"],
  [/^no network adapter has the address (\S+)$/, "noAdapter"],
  [/^device (\d+) did not answer Who-Is within (\d+) s; it may answer only when asked at its address$/, "noWhoIs"],
  [/^a request is still running$/, "busy"],
  [/^only pages served by this bacnetio may use it$/, "foreignPage"],
  [/^cannot read the active alarms \(GetEventInformation\): (.*)$/, "alarmsRefused"],
  [/^a device ID goes from 0 to (\d+)$/, "idRange"],
  [/^at most (\d+) objects at a time$/, "liveMost"],
  [/^priority (\d+) is not between 1 and (\d+)$/, "priorityRange"],
  [/^this object has a priority array: choose a priority$/, "choosePriority"],
  [/^the interval must be at least (\d+) s$/, "intervalMin"],
  [/^another device already has the ID (\d+)$/, "idTaken"],
  [/^(\d+) is already the ID of this device$/, "idOwn"],
  [/^release the values you hold on this device first$/, "releaseFirst"],
  [/^a device behind a router needs its device ID$/, "routedNeedsId"],
  [/^a stop lasts at least one minute$/, "stopMinute"],
  [/^give a name to ask for$/, "noName"],
  [/^cannot tell which kind of value this schedule takes$/, "scheduleKind"],
  [/^this is not a bacnetio site file$/, "notSiteFile"],
  [/^read the devices' points first$/, "readAllFirst"],
  [/^bacnetio has no data folder to save to$/, "noDataFolder"],
  [/^no site named (.*)$/, "noSite"],
  [/^a grouping pattern has at most (\d+) characters$/, "patternTooLong"],
  // the Excel files: what the page sends for the column heads and the title line
  [/^heads must be a JSON object of texts$/, "xlsxHeads"],
  [/^at most (\d+) heads of at most (\d+) characters$/, "xlsxHeadsLong"],
  [/^a title has at most (\d+) characters$/, "xlsxTitleLong"],
  [/^(.*) is not in alarm any more$/, "notInAlarm"],
  [/^the device does not tell when the alarm began, so it cannot be acknowledged from here$/, "noAlarmTime"],
  [/^"(.*)" is not true or false$/, "notBool"],
  [/^"(.*)" is not a whole number$/, "notWhole"],
  [/^"(.*)" is not the number of a choice$/, "notChoice"],
  [/^(.*) cannot be changed here$/, "notHere"],
  [/^a value of kind (.*) cannot be written here$/, "kindNotHere"],
  [/^a week has the days 0 to 6$/, "dayRange"],
  [/^"(.*)" needs a time and a value$/, "needsTimeValue"],
  [/^"(.*)" is no time \(such as 06:30\)$/, "noTime"],
  [/^"(.*)" is no value for this schedule$/, "noScheduleValue"],
  [/^this schedule's kind of value cannot be edited here$/, "scheduleKind"],
  [/^no BACnet device answers at (.*)$/, "noDeviceAt"],
  [/^device (\d+) does not answer through (\S+): (.*)$/, "noRoute"],
  [/^the device at (\S+) has ID (\d+), not (\d+)$/, "wrongId"],
  [/^"(.*)" is not a station address \(such as 12 or 00:1a\)$/, "badStation"],
  [/^reading trend logs of devices behind a router is not supported yet$/, "trendRouter"],
  [/^this object does not hold trend log records$/, "notTrend"],
  [/^reading event logs of devices behind a router is not supported yet$/, "eventRouter"],
  [/^this object does not hold event log records$/, "notEventLog"],
  [/^reading files of devices behind a router is not supported yet$/, "fileRouter"],
  [/^the file is larger than (\d+) MB$/, "fileTooLarge"],
  [/^(.*) is neither warm nor cold$/, "notRestart"],
  [/^(.*) is not an IP address or IP:PORT$/, "notAddress"],
  [/^(.*) is not a subnet like 192\.168\.10\.0\/24$/, "notSubnet"],
  [/^(.*) is not a date and time like 2026-11-01 09:30:05$/, "notDateTime"],
  [/^no network adapter with an IPv4 address is connected$/, "noIpv4"],
  [/^a recording holds at most (\d+) points$/, "recordMost"],
  [/^cannot send the registration to the BBMD at (\S+)$/, "bbmdLocal"],
  [/^plugins need a data folder$/, "pluginNoFolder"],
  [/^no plugin (\S+) is installed$/, "pluginMissing"],
  [/^plugin (.+) (\S+) is installed already$/, "pluginInstalled"],
  [/^plugin (\S+) is switched off$/, "pluginOff"],
  [/^not a bacnetio plugin: the description is missing$/, "pluginNoDescription"],
  [/^not a bacnetio plugin: the description is not closed$/, "pluginOpenDescription"],
  [/^the plugin's description cannot be read: (.*)$/, "pluginUnreadable"],
  [/^the plugin id (.*) must be up to (\d+) small letters, digits and dashes$/, "pluginId"],
  [/^the plugin's name must be 1 to (\d+) characters$/, "pluginName"],
  [/^the plugin's version must be 1 to (\d+) characters$/, "pluginVersion"],
  [/^the plugin's description must be at most (\d+) characters$/, "pluginDescription"],
  [/^the plugin needs plugin API version (\d+); this bacnetio has version (\d+)$/, "pluginApi"],
  [/^the priority array cannot be read, so the value was not released$/, "releaseUnread"],
  [/^priority (\d+) holds (.+) now, not what bacnetio wrote: another system wrote over it, so it was not released$/, "releaseTaken"],
  [/^priority (\d+) is free now: another system released it already$/, "releaseFree"],
];

function stored() {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** A language opened for review with ?lang=, kept like a choice from the menu. */
function previewed() {
  const code = new URLSearchParams(globalThis.location?.search ?? "").get("lang");
  if (!LANGUAGE_FILES[code]) return null;
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // the review then lasts only until the page is reloaded without ?lang=
  }
  return code;
}

/** The first of the system's languages that is in the menu. */
function systemLanguage() {
  const tags = globalThis.navigator?.languages ?? [globalThis.navigator?.language];
  const offered = LANGUAGES.map(([code]) => code);
  return tags.map((tag) => tag?.slice(0, 2).toLowerCase()).find((code) => offered.includes(code));
}

const resolve = (choice) => (LANGUAGE_FILES[choice] ? choice : systemLanguage() ?? "en");

let choice = previewed() ?? stored() ?? "auto";
let language = resolve(choice);
document.documentElement.lang = language;

const file = () => LANGUAGE_FILES[language];

export const currentLanguage = () => language;
/** Locale for dates and times: "de-CH", "en-GB" … */
export const currentLocale = () => file().locale;
/** "auto" or a language code, as chosen in Settings. */
export const languageChoice = () => (LANGUAGE_FILES[choice] ? choice : "auto");

export function setLanguage(code) {
  choice = code;
  language = resolve(code);
  document.documentElement.lang = language;
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // the choice then lasts only until the page is reloaded
  }
}

/** The text for `key` with {placeholders} filled in. */
export function t(key, values = {}) {
  const text = file().texts[key] ?? en.texts[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (_, name) => values[name] ?? "");
}

/** A note from the server ({code, ...values, text}) in the current language. */
export function noteText(note) {
  const own = file().notes[note.code];
  if (!own) return note.text;
  return own.replace(/\{(\w+)\}/g, (_, name) => {
    const value = note[name];
    const taken = name === "reason" && /^another program uses (.+)$/.exec(value ?? "");
    if (taken) return t("port.taken", { address: taken[1] });
    return Array.isArray(value) ? value.join(", ") : value ?? "";
  });
}

/** An error message from the server in the current language; unknown ones stay English. */
export function errorText(message) {
  for (const [pattern, key] of ERROR_PATTERNS) {
    const own = file().errors[key];
    if (own && pattern.test(message)) return message.replace(pattern, own);
  }
  return message;
}

/** A property's value from the server: only the server's note that a long list was cut is ours. */
export const propertyValueText = (text) =>
  String(text ?? "").replace(/… (\d+) more$/, (_, n) => t("list.more", { n }));

/** "Analog value" for a type number, or the number where no language names it. */
export const objectTypeName = (type) => file().objectTypes[type] ?? en.objectTypes[type] ?? String(type);

/** "Analog input 3" in the current language. */
export function objectText(item) {
  const name = file().objectTypes[item.type] ?? en.objectTypes[item.type];
  return name ? `${name} ${item.instance}` : item.object;
}

/** What the server calls an event state ("offnormal", "high limit" or "high-limit") in words. */
const EVENT_STATES = { normal: "normal", fault: "chip.fault", offnormal: "event.offnormal", "high-limit": "prop.high-limit", "low-limit": "prop.low-limit" };
export function eventStateText(text) {
  const key = EVENT_STATES[text.replaceAll(" ", "-")];
  return key ? t(key) : text;
}

/** A value from the server in the current language: "active", "state 2", "21.5 °C" → "21,5 °C". */
export function valueWord(text) {
  if (text === "active" || text === "inactive") return t(text);
  const state = /^state (\d+)$/.exec(text);
  if (state) return t("state", { n: state[1] });
  if (file().decimalComma && /^-?\d+\.\d+( |$)/.test(text)) return text.replace(".", ",");
  return text;
}

/** Translates the server's status words: "fault, overridden". */
export function statusText(text) {
  if (!text) return t("normal");
  const words = { alarm: "chip.alarm", fault: "chip.fault", overridden: "chip.overridden", "out of service": "chip.outOfService" };
  return text.split(", ").map((word) => (words[word] ? t(words[word]).toLowerCase() : word)).join(", ");
}
