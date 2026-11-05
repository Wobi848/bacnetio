// The plugins as released: each built into one file whose script runs as one module.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

test("every plugin builds into one file with a script that parses", () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), "bacnetio-plugins-"));
  try {
    execFileSync(process.execPath, ["plugins/build.mjs", out]);
    const built = fs.readdirSync(out);
    assert.ok(built.includes("kbob.bacnetio-plugin.html"), built.join(", "));
    for (const file of built) {
      const page = fs.readFileSync(path.join(out, file), "utf8");
      assert.match(page, /<script type="application\/json" id="bacnetio-plugin">/, file);
      assert.doesNotMatch(page, /<script src=/, `${file} still loads a file`);
      // the files of a plugin share one scope: two of them declaring the same name breaks it
      const code = page.split('<script type="module">\n')[1].split("</script>")[0];
      const script = path.join(out, `${file}.mjs`);
      fs.writeFileSync(script, code);
      execFileSync(process.execPath, ["--check", script]);
    }
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});
