// Builds each plugin under plugins/ into its single file (docs/plugins.md): the page with its
// scripts put inside, as one module without imports. `node plugins/build.mjs [out-dir]`.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.resolve(process.argv[2] ?? path.join(here, "..", "target", "plugins"));
fs.mkdirSync(out, { recursive: true });

for (const name of fs.readdirSync(here).filter((entry) => fs.statSync(path.join(here, entry)).isDirectory())) {
  const folder = path.join(here, name);
  const page = fs.readFileSync(path.join(folder, "page.html"), "utf8");
  const sources = [...page.matchAll(/<script src="([\w-]+\.js)"><\/script>\n?/g)];
  // the files share one module scope: their imports of each other go, their exports become plain names
  const code = sources.map(([, file]) => fs.readFileSync(path.join(folder, file), "utf8")
    .replace(/^import .*;\n/gm, "")
    .replace(/^export /gm, "")).join("\n");
  const first = sources[0].index;
  const built = page.slice(0, first) + `<script type="module">\n${code}</script>\n` + page.slice(first).replace(/<script src="[\w-]+\.js"><\/script>\n?/g, "");
  const file = path.join(out, `${name}.bacnetio-plugin.html`);
  fs.writeFileSync(file, built);
  console.log(file);
}
