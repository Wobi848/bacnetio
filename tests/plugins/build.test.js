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

test("the build works with Windows line endings too", () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), "bacnetio-plugins-crlf-"));
  try {
    // a copy of the plugins with \r\n, as Git checks them out on Windows
    const copy = path.join(out, "plugins");
    fs.cpSync("plugins", copy, { recursive: true });
    for (const folder of fs.readdirSync(copy).filter((name) => fs.statSync(path.join(copy, name)).isDirectory())) {
      for (const file of fs.readdirSync(path.join(copy, folder))) {
        const at = path.join(copy, folder, file);
        fs.writeFileSync(at, fs.readFileSync(at, "utf8").replace(/\r?\n/g, "\r\n"));
      }
    }
    execFileSync(process.execPath, [path.join(copy, "build.mjs"), path.join(out, "built")]);
    const page = fs.readFileSync(path.join(out, "built", "kbob.bacnetio-plugin.html"), "utf8");
    assert.doesNotMatch(page, /^import /m);
    const script = path.join(out, "kbob.mjs");
    fs.writeFileSync(script, page.split(/<script type="module">\r?\n/)[1].split("</script>")[0]);
    execFileSync(process.execPath, ["--check", script]);
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});
