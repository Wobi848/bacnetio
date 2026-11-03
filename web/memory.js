// What survives a reload of the page in this browser: the search settings, the chosen adapter and
// the open device. The search result itself stays on the server.

const KEY = "bacnetio.session";

export function loadMemory() {
  try {
    return JSON.parse(localStorage.getItem(KEY)) ?? {};
  } catch {
    return {};
  }
}

const WATCH_KEY = "bacnetio.watch";

const TREE_VIEW_KEY = "bacnetio.treeView";
const PROPERTY_NAMES_KEY = "bacnetio.propertyNames";

/** A choice kept in this browser: one of `allowed`, the first unless another was chosen. */
function loadChoice(key, allowed) {
  try {
    const chosen = localStorage.getItem(key);
    return allowed.includes(chosen) ? chosen : allowed[0];
  } catch {
    return allowed[0];
  }
}

function saveChoice(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // without storage the choice lasts until the page is reloaded
  }
}

/** Which view the tree shows: by object type, unless plant parts were chosen. */
export const loadTreeView = () => loadChoice(TREE_VIEW_KEY, ["type", "plant"]);
export const saveTreeView = (view) => saveChoice(TREE_VIEW_KEY, view);

/** Whether the properties show plain words or the BACnet names. */
export const loadPropertyNames = () => loadChoice(PROPERTY_NAMES_KEY, ["plain", "bacnet"]);
export const savePropertyNames = (names) => saveChoice(PROPERTY_NAMES_KEY, names);

export function loadWatch() {
  try {
    return JSON.parse(localStorage.getItem(WATCH_KEY)) ?? [];
  } catch {
    return [];
  }
}

export function saveWatch(watch) {
  try {
    localStorage.setItem(WATCH_KEY, JSON.stringify(watch));
  } catch {
    // without storage the list lasts until the page is reloaded
  }
}

export function saveMemory(state) {
  const memory = { settings: state.settings, adapterIp: state.adapter?.ip ?? null, device: state.device?.instance ?? null };
  try {
    localStorage.setItem(KEY, JSON.stringify(memory));
  } catch {
    // without storage the page starts fresh after a reload, as before
  }
}
