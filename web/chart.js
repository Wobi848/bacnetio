// A small line chart with one axis and a crosshair, for recordings and trend logs.

import { extent, thinned } from "./analysis.js";
import { h } from "./html.js";
import { currentLocale, t, valueWord } from "./i18n.js";

const WIDTH = 600;
const HEIGHT = 150;
const PAD = { top: 12, right: 64, bottom: 24, left: 8 };
/** Spans longer than this label the axis with dates. */
const DAY_MS = 20 * 60 * 60 * 1000;

/** The series of each chart on screen, by id, for the crosshair. */
const shown = new Map();
/** The same for the charts that compare series. */
const shownCompare = new Map();

export const numberText = (value) => valueWord(String(Math.round(value * 100) / 100));

/** Axis labels with as many decimals as it takes for different values to read differently. */
export function axisTexts(values) {
  const distinct = new Set(values).size;
  for (let digits = 2; digits <= 6; digits++) {
    const texts = values.map((value) => valueWord(String(Number(value.toFixed(digits)))));
    if (new Set(texts).size === distinct) return texts;
  }
  return values.map(numberText);
}

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
/** Steps between time marks, round on a clock: the first that gives at most MOST_TICKS marks. */
const TICK_STEPS = [SECOND, 5 * SECOND, 10 * SECOND, 30 * SECOND, MINUTE, 5 * MINUTE, 10 * MINUTE, 15 * MINUTE, 30 * MINUTE,
  HOUR, 2 * HOUR, 3 * HOUR, 6 * HOUR, 12 * HOUR, 24 * HOUR];
const MOST_TICKS = 7;

/** Time marks between `from` and `to` at round local times (10:15, 10:30, …), with their labels. */
export function timeTicks(from, to) {
  const step = TICK_STEPS.find((candidate) => Math.floor((to - from) / candidate) + 1 <= MOST_TICKS) ?? TICK_STEPS.at(-1);
  // round in local time, so an hour mark sits on the hour where the technician is
  const offset = new Date(from).getTimezoneOffset() * MINUTE;
  const ticks = [];
  for (let local = Math.ceil((from - offset) / step) * step; local + offset <= to; local += step) ticks.push(local + offset);
  const options = step >= 24 * HOUR ? { day: "2-digit", month: "2-digit" }
    : step < MINUTE ? { hour: "2-digit", minute: "2-digit", second: "2-digit" }
      : to - from > DAY_MS ? { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" }
        : { hour: "2-digit", minute: "2-digit" };
  return ticks.map((ms) => ({ ms, label: new Date(ms).toLocaleString(currentLocale(), options) }));
}

/** The marks on the time axis of a chart `height` high, kept inside the drawing at both ends. */
function ticksSvg(ticks, x, height) {
  const bottom = height - PAD.bottom;
  return ticks.map(({ ms, label }) => {
    const at = x(ms);
    const anchor = at < PAD.left + 28 ? "start" : at > WIDTH - PAD.right - 28 ? "end" : "middle";
    return `<line x1="${at.toFixed(1)}" x2="${at.toFixed(1)}" y1="${bottom}" y2="${bottom + 4}" class="tick"/><text x="${at.toFixed(1)}" y="${height - 6}" class="axis" text-anchor="${anchor}">${h(label)}</text>`;
  }).join("");
}

/** Binary and multi-state values are states, not numbers: Off and On, State 1 to 3. */
const MOST_STATE_LEVELS = 8;

/**
 * The levels of the value axis and their labels: low and high for a number, every state for a
 * point with `states` ("binary" or "multi").
 */
export function valueLevels(states, low, high) {
  if (states === "binary") return { low: 0, high: 1, levels: [0, 1], labels: [valueWord("inactive"), valueWord("active")] };
  if (states === "multi" && high - low < MOST_STATE_LEVELS) {
    const first = Math.min(low, 1);
    const levels = Array.from({ length: high - first + 1 }, (_, index) => first + index);
    return { low: first, high: Math.max(high, first + 1), levels, labels: levels.map((n) => t("state", { n })) };
  }
  if (low === high) { low -= 1; high += 1; }
  return { low, high, levels: [low, high], labels: axisTexts([low, high]) };
}

/** A value as the tooltip says it: Off or On, State 2, or the number with its unit. */
export function valueLabel(states, value, units) {
  if (states === "binary") return valueWord(value ? "active" : "inactive");
  if (states === "multi") return t("state", { n: value });
  return `${numberText(value)} ${units ?? ""}`.trim();
}

function timeText(ms, withDate) {
  const options = withDate
    ? { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" }
    : { hour: "2-digit", minute: "2-digit", second: "2-digit" };
  return new Date(ms).toLocaleString(currentLocale(), options);
}

export function scale(samples, states = null) {
  const values = samples.map(([, value]) => value).filter((value) => value !== null);
  const { low, high, levels, labels } = valueLevels(states, ...extent(values));
  const first = samples[0][0];
  const last = samples.at(-1)[0];
  const span = Math.max(last - first, 1);
  return {
    low, high, first, last, levels, labels,
    x: (ms) => PAD.left + ((ms - first) / span) * (WIDTH - PAD.left - PAD.right),
    y: (value) => PAD.top + (1 - (value - low) / (high - low)) * (HEIGHT - PAD.top - PAD.bottom),
  };
}

/** Up to this many measurements in view each get a dot; more would be a smear, so only gaps show. */
const DOTS_UP_TO = 80;

/**
 * A mark for every measurement: a dot on the line where there are few, and a red tick on the
 * time axis for each one without answer, so that both life and gaps show on a flat line.
 */
export function marksSvg(samples, s, bottom, color = "") {
  const style = color ? ` style="fill:var(--series-${color})"` : "";
  const dots = samples.length > DOTS_UP_TO ? "" : samples
    .filter(([, value]) => value !== null)
    .map(([ms, value]) => `<circle cx="${s.x(ms).toFixed(1)}" cy="${s.y(value).toFixed(1)}" r="2.5" class="mark"${style}/>`)
    .join("");
  const gaps = samples
    .filter(([, value]) => value === null)
    .map(([ms]) => { const x = s.x(ms).toFixed(1); return `<line x1="${x}" x2="${x}" y1="${bottom - 6}" y2="${bottom}" class="gap-mark"/>`; })
    .join("");
  return dots + gaps;
}

/**
 * A COV point keeps its value until the device reports another, so its line goes on at the last
 * value to `until` (now, while recording). Only the line: no measurement is marked there.
 */
export function heldOn(samples, until) {
  const last = samples.at(-1);
  if (!until || !last || last[1] === null || until <= last[0]) return samples;
  return [...samples, [until, last[1]]];
}

/** The line as SVG path; a gap where there is no value, steps for binary and states. */
export function linePath(samples, s, stepped) {
  let path = "";
  let drawing = false;
  for (const [ms, value] of samples) {
    if (value === null) { drawing = false; continue; }
    const x = s.x(ms).toFixed(1);
    const y = s.y(value).toFixed(1);
    if (!drawing) path += `M${x},${y}`;
    else if (stepped) path += `H${x}V${y}`;
    else path += `L${x},${y}`;
    drawing = true;
  }
  return path;
}

/**
 * The chart for `series` ({ samples: [[ms, value or null]], states, units, label }), or
 * `emptyText` while it has no value. `until`, for a COV point, carries its last value on to then.
 */
export function chartHtml(id, series, emptyText) {
  const samples = series.samples;
  if (!samples.some(([, value]) => value !== null)) return `<p class="hint">${h(emptyText)}</p>`;
  shown.set(id, series);
  const line = heldOn(samples, series.until);
  const s = scale(line, series.states);
  const grid = gridSvg(s.levels, s.labels, s.y);
  const times = ticksSvg(timeTicks(s.first, s.last), s.x, HEIGHT);
  return `<div class="chart" data-chart="${h(id)}">
    <svg viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-label="${h(series.label)}">
      ${grid}${times}
      <path d="${linePath(line, s, Boolean(series.states))}" class="line"/>
      ${marksSvg(samples, s, HEIGHT - PAD.bottom)}
      <g class="cross"><line class="cross-line" y1="${PAD.top}" y2="${HEIGHT - PAD.bottom}"/><circle r="5" class="cross-dot"/></g>
      <rect x="0" y="0" width="${WIDTH}" height="${HEIGHT}" class="hit"/>
    </svg><div class="tip" hidden></div></div>`;
}

/** Lines across at each level, labelled on the right. */
function gridSvg(levels, labels, y) {
  return levels.map((value, index) => {
    const at = y(value).toFixed(1);
    return `<line x1="${PAD.left}" x2="${WIDTH - PAD.right}" y1="${at}" y2="${at}" class="grid"/><text x="${WIDTH - PAD.right + 8}" y="${at}" dy="4" class="axis">${h(labels[index])}</text>`;
  }).join("");
}

/** At the widest a compare chart is drawn (860 px), this keeps it about 220 px high; states need
 *  less room than a number. */
const COMPARE_HEIGHT = 160;
const STATES_HEIGHT = 96;

/**
 * Several series of one unit on one axis over the window `range`, each in its own colour, for
 * comparing them. `series` items: { key, label, samples, states, units, color, until }; the series of
 * one chart share their unit, and so whether they are states.
 */
export function compareChartHtml(id, series, range) {
  const known = series.flatMap((item) => item.samples).filter(([, value]) => value !== null);
  if (!known.length) return "";
  const values = known.map(([, value]) => value);
  const states = series[0].states;
  const chartHeight = states ? STATES_HEIGHT : COMPARE_HEIGHT;
  const axis = valueLevels(states, ...extent(values));
  const { low, high } = axis;
  // a number gets a line in the middle as well
  const levels = states ? axis.levels : [low, (low + high) / 2, high];
  const width = WIDTH - PAD.left - PAD.right;
  const height = chartHeight - PAD.top - PAD.bottom;
  const span = Math.max(range.to - range.from, 1);
  const s = {
    x: (ms) => PAD.left + ((ms - range.from) / span) * width,
    y: (value) => PAD.top + (1 - (value - low) / (high - low)) * height,
  };
  shownCompare.set(id, { series, range, s });
  const grid = gridSvg(levels, states ? axis.labels : axisTexts(levels), s.y);
  const times = ticksSvg(timeTicks(range.from, range.to), s.x, chartHeight);
  const lines = series.map((item) => `<path d="${linePath(thinned(heldOn(item.samples, item.until)), s, Boolean(item.states))}" class="line" style="stroke:var(--series-${item.color})"/>${marksSvg(item.samples, s, chartHeight - PAD.bottom, item.color)}`).join("");
  return `<div class="chart compare" data-compare="${h(id)}">
    <svg viewBox="0 0 ${WIDTH} ${chartHeight}" role="img" aria-label="${h(series.map((item) => item.label).join(", "))}">
      ${grid}${times}${lines}
      <g class="cross"><line class="cross-line" y1="${PAD.top}" y2="${chartHeight - PAD.bottom}"/></g>
      <rect class="zoom" y="${PAD.top}" height="${height}" hidden></rect>
      <rect x="0" y="0" width="${WIDTH}" height="${chartHeight}" class="hit"/>
    </svg><div class="tip" hidden></div></div>`;
}

/** The time under the pointer in a compare chart. */
export function compareTime(element, clientX) {
  const chart = shownCompare.get(element.dataset.compare);
  const box = element.querySelector("svg").getBoundingClientRect();
  const fraction = ((clientX - box.left) / box.width * WIDTH - PAD.left) / (WIDTH - PAD.left - PAD.right);
  return chart.range.from + Math.min(Math.max(fraction, 0), 1) * (chart.range.to - chart.range.from);
}

/** One readout for every series at the time under the pointer, in all compare charts at once. */
export function showCompareCrosshair(event, noAnswer) {
  const element = event.target.closest(".chart.compare");
  if (!element) return;
  const ms = compareTime(element, event.clientX);
  for (const chartElement of document.querySelectorAll(".chart.compare")) {
    const chart = shownCompare.get(chartElement.dataset.compare);
    if (!chart) continue;
    const x = chart.s.x(ms);
    const cross = chartElement.querySelector(".cross");
    cross.classList.add("on");
    cross.querySelector(".cross-line").setAttribute("x1", x);
    cross.querySelector(".cross-line").setAttribute("x2", x);
    if (chartElement !== element) continue;
    const tip = chartElement.querySelector(".tip");
    tip.hidden = false;
    tip.textContent = "";
    const when = document.createElement("div");
    when.textContent = timeText(ms, chart.range.to - chart.range.from > DAY_MS);
    tip.append(when);
    for (const item of chart.series) {
      const nearest = item.samples.length ? item.samples.reduce((best, sample) => (Math.abs(sample[0] - ms) < Math.abs(best[0] - ms) ? sample : best)) : null;
      const line = document.createElement("div");
      line.textContent = `${item.label}: ${nearest && nearest[1] !== null ? valueLabel(item.states, nearest[1], item.units) : noAnswer}`;
      tip.append(line);
    }
    tip.style.left = `${Math.min(Math.max((x / WIDTH) * 100, 22), 78)}%`;
  }
}

export function hideCompareCrosshair() {
  for (const chartElement of document.querySelectorAll(".chart.compare")) {
    chartElement.querySelector(".cross").classList.remove("on");
    chartElement.querySelector(".tip").hidden = true;
  }
}

/** Moves the crosshair of the chart under the pointer to the nearest sample. */
export function showCrosshair(event, noAnswer) {
  const chartElement = event.target.closest(".chart");
  const series = chartElement && shown.get(chartElement.dataset.chart);
  if (!series) return;
  const svg = chartElement.querySelector("svg");
  const box = svg.getBoundingClientRect();
  const s = scale(heldOn(series.samples, series.until), series.states);
  const ms = s.first + ((event.clientX - box.left) / box.width * WIDTH - PAD.left) / (WIDTH - PAD.left - PAD.right) * (s.last - s.first);
  const nearest = series.samples.reduce((best, sample) => (Math.abs(sample[0] - ms) < Math.abs(best[0] - ms) ? sample : best));
  const cross = svg.querySelector(".cross");
  const tip = chartElement.querySelector(".tip");
  const x = s.x(nearest[0]);
  cross.classList.add("on");
  cross.querySelector(".cross-line").setAttribute("x1", x);
  cross.querySelector(".cross-line").setAttribute("x2", x);
  const dot = cross.querySelector(".cross-dot");
  dot.classList.toggle("off", nearest[1] === null);
  if (nearest[1] !== null) { dot.setAttribute("cx", x); dot.setAttribute("cy", s.y(nearest[1])); }
  tip.hidden = false;
  const withDate = s.last - s.first > DAY_MS;
  tip.textContent = `${timeText(nearest[0], withDate)} · ${nearest[1] === null ? noAnswer : valueLabel(series.states, nearest[1], series.units)}`;
  tip.style.left = `${Math.min(Math.max((x / WIDTH) * 100, 15), 85)}%`;
}

export function hideCrosshair(event) {
  const chartElement = event.target.closest(".chart");
  if (!chartElement) return;
  chartElement.querySelector(".cross").classList.remove("on");
  chartElement.querySelector(".tip").hidden = true;
}
