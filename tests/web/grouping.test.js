// A site's own grouping pattern (docs/concept-toolbox.md part 4), without a browser.

import { test } from "node:test";
import assert from "node:assert/strict";

const { compilePattern, groupPoints, sitePattern, splitByPattern } = await import("../../web/grouping.js");

const point = (instance, name) => ({ type: 0, instance, name });

test("a pattern's brackets give the plant part, the rest of the name the point", () => {
  const pattern = compilePattern("^(\\w+_\\w+_\\d+)_");
  const groups = groupPoints([point(1, "ZUL_VEN_01_TT01"), point(2, "ZUL_VEN_01_TT02"), point(3, "ABL_VEN_02_TT01"), point(4, "Outside")], pattern);
  assert.deepEqual(groups.map((group) => group.title), ["ZUL_VEN_01", "ABL_VEN_02", "other"]);
  assert.deepEqual(groups[0].items.map((item) => item.name), ["TT01", "TT02"]);
  assert.equal(groups[2].kind, true, "names the pattern misses go under Other");
  assert.equal(groups[0].key, "part:ZUL_VEN_01", "the same keys as the automatic parts, for the tree");
});

test("several brackets join into one part; without brackets the whole match is the part", () => {
  assert.deepEqual(splitByPattern("B12.AHU1.SupplyTemp", compilePattern("^(\\w+)\\.(\\w+)\\.")), { part: "B12/AHU1", name: "SupplyTemp" });
  assert.deepEqual(splitByPattern("AHU1-SupplyTemp", compilePattern("^AHU\\d")), { part: "AHU1", name: "SupplyTemp" });
  assert.deepEqual(splitByPattern("AHU1", compilePattern("^(AHU\\d)")), { part: "AHU1", name: "AHU1" }, "nothing left keeps the whole name");
  assert.equal(splitByPattern(null, compilePattern("^(x)")), null, "a point without a name");
});

test("a site without a pattern or with a broken one groups by itself", () => {
  assert.equal(sitePattern({ grouping: null }), null);
  assert.equal(sitePattern(null), null);
  assert.equal(sitePattern({ grouping: "([" }), null);
  assert.throws(() => compilePattern("(["), "the panel shows the reason");
  const automatic = groupPoints([point(1, "/a/b/x"), point(2, "/a/b/y"), point(3, "/a/c/z")], sitePattern({ grouping: "([" }));
  assert.deepEqual(automatic.map((group) => group.title), ["b", "c"]);
});
