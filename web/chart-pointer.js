// The pointer on charts: the crosshair, and zooming a compare chart by drawing a span.

import { compareTime, hideCompareCrosshair, hideCrosshair, showCompareCrosshair, showCrosshair } from "./chart.js";
import { t } from "./i18n.js";
import { render, state } from "./store.js";

/** Zooming by drawing a span in a compare chart: where it began, in time and on screen. */
let dragFrom = null;
let dragX = 0;

function drawZoom(event) {
  const svg = event.target.closest(".chart.compare").querySelector("svg");
  const box = svg.getBoundingClientRect();
  const scale = svg.viewBox.baseVal.width / box.width;
  const [left, right] = [dragX, event.clientX].sort((first, second) => first - second);
  const zoom = svg.querySelector(".zoom");
  zoom.hidden = false;
  zoom.setAttribute("x", (left - box.left) * scale);
  zoom.setAttribute("width", (right - left) * scale);
}

document.addEventListener("pointerdown", (event) => {
  const chart = event.target.closest(".chart.compare");
  if (!chart) return;
  dragFrom = compareTime(chart, event.clientX);
  dragX = event.clientX;
});

document.addEventListener("pointerup", (event) => {
  if (dragFrom === null) return;
  const chart = event.target.closest(".chart.compare");
  const from = dragFrom;
  dragFrom = null;
  // a click is not a span
  if (!chart || Math.abs(event.clientX - dragX) < 8) return;
  const to = compareTime(chart, event.clientX);
  state.recZoom = { from: Math.min(from, to), to: Math.max(from, to) };
  state.onChart = false;
  render();
});

document.addEventListener("pointermove", (event) => {
  state.onChart = Boolean(event.target.closest(".chart"));
  if (!state.onChart) return;
  if (event.target.closest(".chart.compare")) {
    showCompareCrosshair(event, t("rec.noAnswer"));
    if (dragFrom !== null) drawZoom(event);
  } else showCrosshair(event, t("rec.noAnswer"));
});
document.addEventListener("pointerout", (event) => {
  if (event.target.closest(".chart") && !event.relatedTarget?.closest?.(".chart")) {
    state.onChart = false;
    if (event.target.closest(".chart.compare")) hideCompareCrosshair();
    else hideCrosshair(event);
  }
});
