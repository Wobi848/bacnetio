// Columns to resize: each table takes its column widths from list.css by itself. A user who drags
// the border right of a column head sets the widths of that table in this browser; a double-click
// hands them back. Not in a list as narrow as a phone, where the columns stack.

import { t } from "./i18n.js";

const KEY = "bacnetio.columns";
/** Below this width of the list, the columns stack as on a phone and keep their own widths. */
const NARROW_LIST = 560;
const NARROWEST = 40;
/** The first column takes what is left, so the table always fits its list. */
const FIRST_LEAST = 120;
const FIRST = `minmax(${FIRST_LEAST}px, 1fr)`;
/** A column no head sorts, of a fixed width: *Remove* in the watch list. */
const REMOVE = "remove";
/** A column the user has not set where a layout shows it: a share of what is left. */
const UNSET = "minmax(60px, 1fr)";
/**
 * Which columns each table shows where, by the keys of their heads: the point list hides its
 * descriptions when it has none, and below 761 px (list.css). The widths are kept per column, so a
 * width set with the details open holds with them closed.
 */
const LAYOUTS = {
  devices: [["(min-width: 561px)", ".rows .row.devices", ["name", "id", "address", "vendor", "objects"]]],
  points: [
    ["(min-width: 761px)", ".rows:not(.no-desc) .row.points", ["name", "description", "id", "type", "value", "state"]],
    ["(min-width: 761px)", ".rows.no-desc .row.points", ["name", "id", "type", "value", "state"]],
    ["(min-width: 561px) and (max-width: 760px)", ".rows .row.points", ["name", "id", "type", "value", "state"]],
  ],
  problems: [["(min-width: 561px)", ".rows .row.problem:not(.watch)", ["device", "name", "value", "state"]]],
  watch: [["(min-width: 561px)", ".rows .row.problem.watch", ["device", "name", "value", "state", REMOVE]]],
};

function track(set, column, index) {
  if (index === 0) return FIRST;
  if (column === REMOVE) return "104px";
  return set[column] ? `${set[column]}px` : UNSET;
}

/** The border right of a column head, to drag. */
export const columnBorder = (list, key) =>
  `<span class="col-border" data-list="${list}" data-key="${key}" role="separator" aria-orientation="vertical" title="${t("column.resize")}"></span>`;

function load() {
  try {
    return JSON.parse(localStorage.getItem(KEY)) ?? {};
  } catch {
    return {};
  }
}

function save(widths) {
  try {
    localStorage.setItem(KEY, JSON.stringify(widths));
  } catch {
    // without storage the widths last until the page is reloaded
  }
}

let widths = {};

function apply() {
  let style = document.getElementById("column-widths");
  if (!style) {
    style = document.createElement("style");
    style.id = "column-widths";
    document.head.append(style);
  }
  style.textContent = Object.entries(widths).flatMap(([list, set]) => (LAYOUTS[list] ?? []).map(([when, rows, columns]) =>
    `@container ${when} { ${rows} { grid-template-columns: ${columns.map((column, index) => track(set, column, index)).join(" ")}; } }`)).join("\n");
}

let dragging = null;

/** Sets the remembered widths and lets the borders be dragged; once, when the page starts. */
export function installColumns() {
  widths = load();
  apply();

  document.addEventListener("pointerdown", (event) => {
    const border = event.target.closest?.(".col-border");
    if (!border || event.button !== 0) return;
    event.preventDefault();
    const head = border.closest(".row.head");
    if (head.closest(".listwrap").getBoundingClientRect().width <= NARROW_LIST) return;
    // the tracks as laid out, not the heads: a head aligned to the end is narrower than its column
    const tracks = getComputedStyle(head).gridTemplateColumns.split(" ").map((width) => Math.round(parseFloat(width)));
    const keys = [...head.children].filter((cell) => cell.offsetParent !== null).map((cell) => cell.querySelector(".col-border")?.dataset.key);
    const column = keys.indexOf(border.dataset.key);
    // the first column is the flexible one: its border moves the next column instead
    const changed = column === 0 ? 1 : column;
    if (column < 0 || !keys[changed]) return;
    const list = border.dataset.list;
    const set = { ...widths[list] };
    keys.forEach((key, index) => { if (key && index > 0) set[key] = tracks[index]; });
    widths[list] = set;
    dragging = { list, key: keys[changed], start: tracks[changed], room: tracks[0] - FIRST_LEAST, sign: column === 0 ? -1 : 1, from: event.clientX };
    document.documentElement.classList.add("resizing");
  });

  document.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    const { list, key, start, room, sign, from } = dragging;
    // the first column gives the room, down to its least; the row never grows out of its list
    widths[list][key] = Math.min(start + room, Math.max(NARROWEST, start + sign * (event.clientX - from)));
    apply();
  });

  document.addEventListener("pointerup", () => {
    if (!dragging) return;
    dragging = null;
    document.documentElement.classList.remove("resizing");
    save(widths);
  });

  document.addEventListener("dblclick", (event) => {
    const border = event.target.closest?.(".col-border");
    if (!border) return;
    delete widths[border.dataset.list];
    apply();
    save(widths);
  });
}
