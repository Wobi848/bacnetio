// Changing many points at once (docs/concept-toolbox.md part 5): what each point takes, without a
// browser. The server checks every write again as a single one.

import { test } from "node:test";
import assert from "node:assert/strict";

globalThis.document = { documentElement: {} };
const { plan, refusal, toWrite } = await import("../../web/change-many.js");

const point = (type, instance, extra = {}) => ({ type, instance, name: `P${instance}`, value: "1", status: "", description: "", state_texts: [], ...extra });

test("a point of a kind without the property is left out", () => {
  assert.equal(refusal("covIncrement", point(3, 1), "0.5"), "many.notThis", "a binary input has no COV increment");
  assert.equal(refusal("value", point(0, 1), "20"), "many.notThis", "an input's value is not commanded");
  assert.equal(refusal("description", point(17, 1), "Office"), null, "every object has a description");
});

test("a value the point cannot take is left out before anything is written", () => {
  assert.equal(refusal("value", point(1, 1), "21,5"), null, "a decimal comma is a number");
  assert.equal(refusal("value", point(1, 1), "warm"), "many.needsNumber");
  assert.equal(refusal("value", point(4, 1), "Active"), null);
  assert.equal(refusal("value", point(4, 1), "2"), "many.needsBinary");
  const mode = point(19, 1, { state_texts: ["Off", "Day", "Night"] });
  assert.equal(refusal("value", mode, "3"), null);
  assert.equal(refusal("value", mode, "4"), "many.needsState", "beyond its state texts");
  assert.equal(refusal("highLimit", point(2, 1), ""), "many.needsNumber", "an empty field is no number");
  assert.equal(refusal("release", point(1, 1), ""), null, "releasing takes no value");
});

test("only the points that change are written", () => {
  const rows = plan("outOfService", [point(0, 1, { status: "out of service" }), point(0, 2), point(17, 3)], "true");
  assert.deepEqual(rows.map((row) => [row.refused, row.unchanged]), [[null, true], [null, false], ["many.notThis", false]]);
  assert.deepEqual(toWrite(rows).map((row) => row.point.instance), [2]);
});
