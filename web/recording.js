// The recording: one small chart per point (small multiples, one axis each), the latest value large.

import { byUnit, extent, figures, inWindow } from "./analysis.js";
import { chartHtml, compareChartHtml, numberText, valueLabel } from "./chart.js";
import { excelLink } from "./excel.js";
import { h } from "./html.js";
import { currentLocale, objectText, t } from "./i18n.js";

export const INTERVALS = [5, 10, 30, 60];
const BINARY = [3, 4, 5];
const MULTISTATE = [13, 14, 19];
/** Binary and multi-state points are drawn as steps between their states. */
const statesOf = (type) => (BINARY.includes(type) ? "binary" : MULTISTATE.includes(type) ? "multi" : null);

export const recordingKey = (point) => `${point.device}:${point.type}:${point.instance}`;

/** The page is polled every 2 s: a measurement younger than that is new on this drawing. */
const FRESH_MS = 2500;

/** Poll or COV for one point; a refused COV says so. */
function modeHtml(point) {
  const key = recordingKey(point);
  const button = (mode, label) => `<button type="button" data-act="rec-mode" data-key="${key}" data-mode="${mode}" aria-pressed="${point.mode === mode}">${t(label)}</button>`;
  const refused = point.mode === "cov" && point.cov_refused ? `<span class="chip warn" title="${t("rec.covRefusedHint")}">${t("rec.covRefused")}</span>` : "";
  return `<span class="rec-mode"><span class="seg">${button("poll", "rec.poll")}${button("cov", "rec.cov")}</span>${refused}</span>`;
}

/** When the last measurement came and how often, flashing once when it is new: a flat line still
 *  shows life. */
function lastMeasuredHtml(point, interval) {
  const last = point.samples.at(-1);
  if (!last) return "";
  const time = new Date(last[0]).toLocaleTimeString(currentLocale());
  const fresh = Date.now() - last[0] < FRESH_MS ? " fresh" : "";
  const how = point.mode === "cov" && !point.cov_refused ? t("rec.measuredCov", { time }) : t("rec.measured", { time, interval });
  return `<span class="measured${fresh}">${how}</span>`;
}

function latest(point) {
  const last = [...point.samples].reverse().find(([, value]) => value !== null);
  if (!last) return "";
  const states = statesOf(point.type);
  if (states) return h(valueLabel(states, last[1]));
  return `${h(numberText(last[1]))}<small>${h(point.units ?? "")}</small>`;
}

/** What an empty chart says: waiting means something runs. */
const emptyText = (running, point) => t(!running ? "rec.notStartedShort" : point.samples.length ? "rec.noValues" : "rec.waiting");

/** Until when a point's line runs: a COV point holds its last value until now, while recording. */
const heldUntil = (running, point) => (running && point.mode === "cov" ? Date.now() : null);

/** One point (Chart): its name and the latest value as the headline, the line, then when it was
 *  measured and its tools. */
function card(recording, point, index) {
  const running = recording.running;
  const series = { samples: point.samples, states: statesOf(point.type), units: point.units, label: point.name || objectText(point), until: heldUntil(running, point) };
  return `<section class="card">
    <div class="series-head"><div><h2>${h(point.name || objectText(point))}</h2><p class="sub">${h(objectText(point))} · ${t("device", { n: point.device })}</p></div>
      <div class="value mono">${latest(point)}</div></div>
    ${chartHtml(`recording-${index}`, series, emptyText(running, point))}
    <div class="rec-meta">${lastMeasuredHtml(point, recording.interval_s)}${modeHtml(point)}
      <button type="button" class="quiet" data-act="rec-remove" data-key="${recordingKey(point)}">${t("rec.remove")}</button></div></section>`;
}

/** Rolling windows of the last minutes; a window drawn with the pointer stands above them. */
const WINDOWS = [[null, "rec.all"], [60, "rec.last60"], [15, "rec.last15"], [5, "rec.last5"]];
const MOST_COLORS = 8;

/** A colour stays with its point, whatever else is added or removed. */
function colorOf(state, key) {
  if (!state.recColors[key]) {
    const used = new Set(Object.values(state.recColors));
    state.recColors[key] = Array.from({ length: MOST_COLORS }, (_, index) => index + 1).find((color) => !used.has(color)) ?? MOST_COLORS;
  }
  return state.recColors[key];
}

function rangeOf(state, points, until) {
  const times = points.flatMap((point) => point.samples.map(([ms]) => ms));
  if (!times.length) return null;
  const [earliestKnown, latestKnown] = extent(times);
  const latest = Math.max(latestKnown, until ?? 0);
  if (state.recZoom) return state.recZoom;
  const earliest = state.recWindow ? latest - state.recWindow * 60000 : earliestKnown;
  return { from: earliest, to: Math.max(latest, earliest + 1) };
}

const figureText = (value, units) => (value === null ? "–" : `${numberText(value)}${units ? ` ${units}` : ""}`);

function figuresRow(item, interval) {
  const found = figures(item.samples, item.stepped);
  const units = item.stepped ? "" : item.units ?? "";
  const stepped = item.stepped && found.onShare !== null ? `${Math.round(found.onShare * 100)} % · ${t("rec.changes", { n: found.changes })}` : "";
  return `<tr><th scope="row"><span class="swatch" style="background:var(--series-${item.color})"></span>${h(item.label)}
    <div class="rec-meta">${modeHtml(item.point)}${lastMeasuredHtml(item.point, interval)}<button type="button" class="quiet" data-act="rec-remove" data-key="${item.key}">${t("rec.removeShort")}</button></div></th>
    <td class="mono">${h(figureText(found.last, units))}</td><td class="mono">${h(figureText(found.low, units))}</td><td class="mono">${h(figureText(found.high, units))}</td>
    <td class="mono">${h(item.stepped ? stepped : figureText(found.mean, units))}</td><td class="mono">${found.count}${found.gaps ? ` <span class="chip warn">${t("rec.gaps", { n: found.gaps })}</span>` : ""}</td></tr>`;
}

/** One time axis for all, a chart per unit, and the figures of each series in the window. */
function compareHtml(state, points) {
  // without samples yet the series stand there all the same, to be removed or waited for
  const running = state.recording.running;
  const range = rangeOf(state, points, Math.max(0, ...points.map((point) => heldUntil(running, point) ?? 0))) ?? { from: 0, to: 1 };
  const series = points.map((point) => ({
    key: recordingKey(point),
    label: point.name || objectText(point),
    units: point.units,
    states: statesOf(point.type),
    stepped: Boolean(statesOf(point.type)),
    color: colorOf(state, recordingKey(point)),
    samples: inWindow(point.samples, range),
    until: heldUntil(running, point),
    point,
  }));
  const charts = byUnit(series).map((group, index) => {
    const unit = group[0].stepped ? t("rec.state") : group[0].units || t("rec.noUnit");
    const legend = group.map((item) => `<span class="legend-item"><span class="swatch" style="background:var(--series-${item.color})"></span>${h(item.label)}</span>`).join("");
    const chart = compareChartHtml(`compare-${index}`, group, range) || `<p class="hint">${t(state.recording.running ? "rec.noValues" : "rec.notStartedShort")}</p>`;
    return `<section class="card"><h2>${h(unit)}</h2><div class="legend">${legend}</div>${chart}</section>`;
  }).join("");
  const rows = series.map((item) => figuresRow(item, state.recording.interval_s)).join("");
  return `${charts}<section class="card"><h2>${t("rec.figures")}</h2><div class="table-scroll"><table class="stats"><thead><tr><th>${t("col.name")}</th><th>${t("rec.last")}</th><th>${t("rec.low")}</th><th>${t("rec.high")}</th><th>${t("rec.mean")}</th><th>${t("rec.samples")}</th></tr></thead><tbody>${rows}</tbody></table></div></section>`;
}

/** The state in a word with a dot, in the toolbar: running or stopped, how many points and values. */
function stateHtml(recording, points) {
  const count = Math.max(...points.map((point) => point.samples.length));
  const pointCount = points.length === 1 ? t("rec.pointOne") : t("rec.points", { n: points.length });
  const word = recording.running ? t("rec.running") : t("rec.stopped");
  return `<span class="state rec-state"><span class="dot${recording.running ? " on" : ""}"></span>${h(`${word} · ${pointCount} · ${t("rec.samplesCount", { count })}`)}</span>`;
}

export function viewRecording(state) {
  const recording = state.recording;
  const points = recording?.points ?? [];
  // while it runs, the interval it runs at; before, the one chosen for the start
  const interval = recording?.running ? recording.interval_s : state.recordInterval;
  const lead = t("rec.lead", { interval });
  if (!points.length) {
    return {
      title: t("tab.recording"),
      lead,
      next: null,
      // the same frame as the empty watch list and site problems
      html: `<div class="split single"><section class="listwrap"><div class="rows"><div class="empty"><h2>${t("rec.emptyTitle")}</h2><span>${t("rec.emptyText")}</span></div></div></section></div>`,
    };
  }
  const intervals = INTERVALS.map((seconds) => `<button type="button" data-act="rec-interval" data-seconds="${seconds}" aria-pressed="${seconds === interval}">${t("rec.every", { seconds })}</button>`).join("");
  const offset = -new Date().getTimezoneOffset();
  const windows = WINDOWS.map(([minutes, label]) => `<button type="button" data-act="rec-window" data-minutes="${minutes ?? ""}" aria-pressed="${state.recWindow === minutes && !state.recZoom}">${t(label)}</button>`).join("");
  const zoom = state.recZoom ? `<button type="button" class="tonal" data-act="rec-zoom-out">${t("rec.zoomOut")}</button>` : "";
  const views = [["cards", "rec.oneByOne"], ["compare", "rec.compare"]].map(([view, label]) => `<button type="button" data-act="rec-view" data-view="${view}" aria-pressed="${state.recView === view}">${t(label)}</button>`).join("");
  // while it runs, stopping is a tool: the page has no step to push then
  const stop = { act: "rec-stop", label: t("rec.stop"), stays: true };
  return {
    title: t("tab.recording"),
    lead,
    next: recording.running ? null : { act: "rec-start", label: t("rec.start") },
    also: recording.running ? stop : null,
    html: `<div class="split single"><section class="listwrap">
      <div class="toolbar"><span class="sub">${t("rec.interval")}</span><span class="seg">${intervals}</span>
        <span class="seg">${views}</span><span class="seg">${windows}</span>${zoom}
        <div class="end">${stateHtml(recording, points)}
          <button type="button" class="quiet" data-act="rec-clear"${recording.running ? " disabled" : ""}>${t("rec.clear")}</button>
          <a class="quiet" href="/api/recording.csv?offset_min=${offset}" download>${t("rec.export")}</a>
          ${excelLink(state, "/api/recording.xlsx", t("xlsx.recExport"), [t("tab.recording")])}</div></div>
      ${recording.running ? "" : `<div class="strips"><div class="strip info">${t("rec.notStarted", { interval })}</div></div>`}
      <div class="page" data-keep="recording">${state.recView === "compare" ? `<div class="analysis">${compareHtml(state, points)}</div>` : `<div class="cards two">${points.map((point, index) => card(recording, point, index)).join("")}</div>`}</div></section></div>`,
  };
}
