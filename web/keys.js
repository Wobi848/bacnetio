// The keyboard: Escape closes, arrows move through lists and menus, digits choose the areas.

import { find, locate, selectDevice } from "./device-actions.js";
import { selectPoint } from "./object-actions.js";
import { AREAS, render, state } from "./store.js";
import { closeMenu, closeTopmost, enter, openPrint, showDevice, showSite } from "./navigation.js";
import { FIELDS } from "./fields.js";

/** Moves the chosen row of the list on screen by that many rows. */
function moveSelection(step) {
  const rows = [...document.querySelectorAll(".rows .row[data-device], .rows .row[data-point]")];
  if (!rows.length) return;
  const index = rows.findIndex((row) => row.classList.contains("active"));
  const next = rows[Math.max(0, Math.min(rows.length - 1, index + step))];
  next.scrollIntoView({ block: "nearest" });
  if (next.dataset.device) selectDevice(Number(next.dataset.device));
  else selectPoint(next.dataset.point);
}

const PAGE_ROWS = 10;
const MOVE_KEYS = { ArrowDown: 1, ArrowUp: -1, PageDown: PAGE_ROWS, PageUp: -PAGE_ROWS, Home: -Infinity, End: Infinity };

const typing = () => document.activeElement?.matches?.("input, select, textarea");

/**
 * The Android back key: one step back, from the details to the device to the device list to the
 * workspace. When there is no step left, the app asks whether to leave and warns of held values.
 */
window.bacnetioBack = () => {
  if (closeTopmost()) render();
  else if (state.area === "workspace" && state.device !== null) showSite();
  else if (state.area !== "workspace") enter("workspace");
  else return JSON.stringify({ stepped: false, held: state.held.length });
  return JSON.stringify({ stepped: true, held: state.held.length });
};

/** Up and down move through a list menu, as through the rows of a list. */
function moveInMenu(event) {
  const menu = event.target.closest(".menu[role=listbox], .menu[role=menu]");
  if (!menu || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return false;
  const items = [...menu.querySelectorAll("button, a")];
  const at = items.indexOf(document.activeElement);
  const next = event.key === "ArrowDown" ? Math.min(at + 1, items.length - 1) : Math.max(at - 1, 0);
  items[next]?.focus();
  return true;
}

/** Tab stays inside an open dialog. */
function keepInDialog(event) {
  const box = document.querySelector(".modal-box");
  if (!box || event.key !== "Tab") return false;
  const fields = [...box.querySelectorAll("button, select, input, a[href]")].filter((field) => !field.disabled);
  const [first, last] = [fields[0], fields[fields.length - 1]];
  if (!box.contains(document.activeElement)) first?.focus();
  else if (event.shiftKey && document.activeElement === first) last.focus();
  else if (!event.shiftKey && document.activeElement === last) first.focus();
  else return false;
  return true;
}

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    if (state.menu) closeMenu();
    else if (closeTopmost()) render();
    return;
  }
  if (moveInMenu(event) || keepInDialog(event)) {
    event.preventDefault();
    return;
  }
  if (event.key === "Enter" && typing()) {
    if (event.target.id.startsWith("locate-")) locate();
    else if (FIELDS[event.target.id]) find();
    return;
  }
  // the browser would print the list with bacnetio around it: the print view is made for paper
  if ((event.ctrlKey || event.metaKey) && event.key === "p" && state.area === "workspace" && state.device && state.points && !state.printing) {
    event.preventDefault();
    openPrint();
    window.print();
    return;
  }
  if (typing() || event.ctrlKey || event.metaKey || event.altKey) return;
  // 1 to 6 lie side by side on the keyboard, 0 sits apart
  if (/^[1-6]$/.test(event.key)) { enter(AREAS[Number(event.key) - 1]); return; }
  if (event.key === "?") { enter("help"); return; }
  if (event.key === "/") {
    event.preventDefault();
    document.getElementById("site-search")?.focus();
    return;
  }
  if (event.key === "f") {
    event.preventDefault();
    document.getElementById("filter")?.focus();
    return;
  }
  if (MOVE_KEYS[event.key]) {
    event.preventDefault();
    moveSelection(MOVE_KEYS[event.key]);
    return;
  }
  if (event.key === "Enter" && state.area === "workspace" && !state.device && state.selectedDevice !== null) showDevice(state.selectedDevice);
});
