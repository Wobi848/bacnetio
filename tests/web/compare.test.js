// Two devices compared by point name (docs/concept-toolbox.md part 2), without a browser.

import { test } from "node:test";
import assert from "node:assert/strict";

globalThis.document = { documentElement: {} };
const { COMPARE_VIEWS, matchPoints, sharedStart } = await import("../../web/compare.js");

const point = (type, instance, name, value = "1", extra = {}) => ({ type, instance, name, value, units: null, status: "", ...extra });

test("the start all of a device's names share is cut at a separator", () => {
  assert.equal(sharedStart(["AHU-01 Supply temp", "AHU-01 Supply fan", "AHU-01 Return temp"]), "AHU-01 ");
  assert.equal(sharedStart(["/21/000/09/S1", "/21/000/10/S2"]), "/21/000/");
  assert.equal(sharedStart(["Supply", "Return"]), "");
  assert.equal(sharedStart(["Only one"]), "", "one name shares nothing with others");
});

test("points meet by name without the device's own start, and differences are named", () => {
  const left = [point(0, 1, "AHU-01 Supply temp", "21.4"), point(1, 1, "AHU-01 Valve", "35"), point(2, 1, "AHU-01 Setpoint", "20")];
  const right = [point(0, 7, "AHU-02 Supply temp", "21.4"), point(1, 3, "AHU-02 Valve", "40"), point(4, 1, "AHU-02 Fan", "1")];
  const rows = matchPoints(left, right);
  assert.deepEqual(rows.map((row) => row.name), ["Supply temp", "Valve", "Setpoint", "Fan"]);
  assert.deepEqual(rows[0].differs, [], "another number at the same name is the same point");
  assert.equal(rows[0].right.instance, 7);
  assert.deepEqual(rows[1].differs, ["value"]);
  assert.equal(rows[2].right, null, "only on the left");
  assert.equal(rows[3].left, null, "only on the right");
  assert.deepEqual(Object.fromEntries(Object.entries(COMPARE_VIEWS).map(([view, keep]) => [view, rows.filter(keep).length])), { all: 4, differ: 1, onlyLeft: 1, onlyRight: 1 });
});

test("type, unit and state differ too; two points of one name are matched in order", () => {
  const rows = matchPoints(
    [point(0, 1, "A Temp", "1", { units: "°C" }), point(3, 1, "A Alarm", "0", { status: "alarm" }), point(0, 2, "A Temp", "2")],
    [point(2, 1, "B Temp", "1", { units: "K" }), point(3, 1, "B Alarm", "0"), point(0, 2, "B Temp", "2")],
  );
  assert.deepEqual(rows[0].differs, ["type", "unit"]);
  assert.deepEqual(rows[1].differs, ["state"]);
  assert.equal(rows[2].right.instance, 2, "the second of a name meets the second");
  assert.deepEqual(rows[2].differs, []);
});
