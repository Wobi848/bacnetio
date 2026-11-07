// Browser tests of the interface: two simulated devices on loopback addresses, per test a fresh
// bacnetio with its own data folder.
//
//   cargo build --all-targets && node tests/ui/run.mjs     all tests
//   node tests/ui/run.mjs layout                           only tests with "layout" in their name
//
// Needs Playwright (`npm i -D playwright && npx playwright install chromium`) or the path to an
// existing package in PLAYWRIGHT. Other browser: BACNETIO_TEST_BROWSER=firefox or webkit.

import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SIM_PORT, SIMULATORS } from "./helpers.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const exe = process.platform === "win32" ? ".exe" : "";
const bin = path.join(root, `target/debug/bacnetio${exe}`);
const require = createRequire(import.meta.url);
const engine = process.env.BACNETIO_TEST_BROWSER || "chromium";
let browserType;
try {
  browserType = require(process.env.PLAYWRIGHT || "playwright")[engine];
} catch {
  console.error("Playwright is missing: npm i -D playwright, or PLAYWRIGHT=<path/to/node_modules/playwright>");
  process.exit(2);
}
if (!browserType) {
  console.error(`Unknown browser "${engine}": chromium, firefox or webkit`);
  process.exit(2);
}
for (const file of [bin, ...SIMULATORS.map(([example]) => path.join(root, `target/debug/examples/${example}${exe}`))]) {
  if (!fs.existsSync(file)) {
    console.error(`${file} is missing: run cargo build --all-targets first`);
    process.exit(2);
  }
}

const filter = process.argv[2] || "";
const tests = [];
for (const file of fs.readdirSync(here).filter((name) => name.endsWith(".test.mjs")).sort()) {
  const module = await import(path.join(here, file));
  tests.push({ file, ...module.default });
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

const simulators = SIMULATORS.map(([example, address]) =>
  spawn(path.join(root, `target/debug/examples/${example}${exe}`), [address, String(SIM_PORT)], { stdio: "ignore" }));

/**
 * The MS/TP device on a pseudo-terminal (Linux only): bacnetio lists the terminal it prints as a
 * serial port, through BACNETIO_TEST_SERIAL.
 */
async function startMstpSimulator() {
  if (process.platform !== "linux") return null;
  const started = spawn(path.join(root, "target/debug/examples/simulated_mstp"), ["5"], { stdio: ["ignore", "pipe", "inherit"] });
  const terminal = await new Promise((found) => {
    let printed = "";
    started.stdout.on("data", (chunk) => {
      printed += chunk;
      const name = printed.match(/on (\/dev\/\S+) \(/);
      if (name) found(name[1]);
    });
    started.once("exit", () => found(null));
  });
  if (terminal) process.env.BACNETIO_TEST_SERIAL = terminal;
  return started;
}
const mstpSimulator = await startMstpSimulator();

/** Starts bacnetio on `port` with the data folder `dir` and waits until its API answers. */
async function launch(port, dir) {
  const started = spawn(bin, ["--web-port", String(port), "--no-browser", "--data-dir", dir, "--test-loopback"], { stdio: "ignore" });
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      await fetch(`http://127.0.0.1:${port}/api/info`);
      break;
    } catch {
      await sleep(100);
    }
  }
  return started;
}

/** A bacnetio with an empty data folder; `restart` keeps the folder, as after closing the program. */
async function startServer(port) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bacnetio-ui-"));
  let running = await launch(port, dir);
  const base = `http://127.0.0.1:${port}`;
  const api = async (route, method = "GET", body) => {
    const response = await fetch(`${base}/api/${route}`, {
      method,
      headers: { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const data = await response.json().catch(() => null);
    if (!response.ok) throw new Error(`${method} ${route}: ${data?.error || response.status}`);
    return data;
  };
  return {
    base,
    api,
    async restart() {
      running.kill();
      await new Promise((done) => running.once("exit", done));
      running = await launch(port, dir);
    },
    stop() {
      running.kill();
      fs.rmSync(dir, { recursive: true, force: true });
    },
  };
}

const browser = await browserType.launch();
let failed = 0;
let port = Number(process.env.BACNETIO_TEST_PORT) || 18400;
for (const test of tests.filter((candidate) => candidate.name.includes(filter) || candidate.file.includes(filter))) {
  const server = await startServer(port++);
  const context = await browser.newContext({ viewport: { width: 1500, height: 900 }, acceptDownloads: true, locale: "en-US" });
  // the look for a new version would go to GitHub: tests stay offline, with nothing newer
  await context.route("**/api/update", (route) => route.fulfill({ json: { current: "0.1.0", latest: null, newer: false } }));
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => {
    // WebKit reports requests aborted by a reload as errors, even when they are caught
    if (engine === "webkit" && /due to access control checks|Load failed/.test(`${error.message}`)) return;
    errors.push(error.stack || error.message);
  });
  // confirmations stand at the button; a browser window (alert/confirm/prompt) is an error
  page.on("dialog", (dialog) => {
    errors.push(`browser dialog: ${dialog.message()}`);
    dialog.dismiss();
  });
  const started = Date.now();
  try {
    if (test.setup) await test.setup(server.api);
    // a second bacnetio with a folder of its own stands for another system on the same plant
    const others = [];
    const another = async () => {
      const other = await startServer(port++);
      others.push(other);
      return other;
    };
    try {
      await test.run({ page, api: server.api, base: server.base, context, restart: server.restart, another });
    } finally {
      for (const other of others) other.stop();
    }
    if (errors.length) throw new Error(`errors in the browser: ${errors.join(" | ")}`);
    console.log(`✓ ${test.name} (${((Date.now() - started) / 1000).toFixed(1)} s)`);
  } catch (error) {
    failed++;
    console.log(`✗ ${test.name}\n    ${String(error.stack || error).split("\n").slice(0, 30).join("\n    ")}`);
    await page.screenshot({ path: path.join(os.tmpdir(), `bacnetio-ui-${test.file}.png`) }).catch(() => {});
  } finally {
    await context.close();
    server.stop();
  }
}
await browser.close();
for (const simulator of simulators) simulator.kill();
mstpSimulator?.kill();
console.log(failed ? `${failed} failed` : "all tests passed");
process.exit(failed ? 1 : 0);
