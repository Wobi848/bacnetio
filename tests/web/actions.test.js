// Every button names its action by data-act: one with no entry in actions.js would do nothing.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";

globalThis.document = { documentElement: {}, addEventListener() {} };
globalThis.window = { addEventListener() {} };
const { actions } = await import("../../web/actions.js");

test("every button's data-act has an action", () => {
  const sources = readdirSync("web").filter((file) => file.endsWith(".js") || file.endsWith(".html"))
    .map((file) => readFileSync(`web/${file}`, "utf8")).join("\n");
  // written out in the markup, or given as `act: "…"` to markup that is built from it
  const named = new Set([...sources.matchAll(/(?:data-act="|\bact: ")([\w-]+)"/g)].map((match) => match[1]));
  assert.ok(named.size > 100, `found ${named.size}`);
  assert.deepEqual([...named].filter((name) => !actions[name]), []);
});
