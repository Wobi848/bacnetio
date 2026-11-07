// The Excel files' heads (docs/concept-excel.md): every column head and sheet name the server
// writes has a text in every language, so no head stays English unnoticed.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

globalThis.document = { documentElement: {} };
const { HEADS } = await import("../../web/excel.js");
const { LANGUAGE_FILES } = await import("../../web/i18n.js");

/** The heads of src/export.rs: its column lists and the heads it adds, and the sheet names. */
async function serverHeads() {
  const exported = await readFile("src/export.rs", "utf8");
  const lists = [...exported.matchAll(/const \w*COLUMNS: \[&str; \d+\] = \[([^\]]*)\]/g)].map((match) => match[1]).join(",");
  const heads = [...lists.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  heads.push(...[...exported.matchAll(/columns\.extend\(\[([^\]]*)\]\)/g)].flatMap((match) => [...match[1].matchAll(/"([^"]+)"/g)].map((head) => head[1])));
  heads.push(...[...exported.matchAll(/vec!\["(Time)"\.to_string\(\)\]/g)].map((match) => match[1]));
  const sheets = await readFile("src/server/workbooks.rs", "utf8");
  heads.push(...[...sheets.matchAll(/named\(&labels, "([^"]+)"\)/g)].map((match) => match[1]));
  return [...new Set(heads)];
}

test("every head and sheet name the server writes has a text key", async () => {
  const heads = await serverHeads();
  assert.ok(heads.length > 20, `found ${heads.length}`);
  assert.deepEqual(heads.filter((head) => !HEADS[head]), []);
});

test("every Excel head is translated in every language", async () => {
  for (const [language, file] of Object.entries(LANGUAGE_FILES)) {
    const texts = file.texts;
    assert.deepEqual(Object.values(HEADS).filter((key) => !texts[key]), [], language);
  }
});
