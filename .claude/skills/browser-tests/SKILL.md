---
name: browser-tests
description: "Run bacnetio's full check set (cargo fmt/test/clippy, the UI logic tests, the browser tests in Chromium, Firefox and WebKit) against the simulators, and take screenshots of the interface at phone and desktop width, light and dark. Use this before saying a bacnetio change is done, after any change in web/, when a browser test fails, or when the maintainer asks to test, check a screen, take a screenshot or \"schau dir das an\" — even if they only say \"läuft alles?\"."
---

# Browser tests and screenshots for bacnetio

Everything runs against the simulators in `examples/` on loopback addresses. Never against a real
network or device.

## Full check set (what CI runs, plus the browsers)

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --no-default-features -- -D warnings   # the build without BACnet/SC
node --test tests/web/*.test.js   # as CI (.github/workflows/build.yml)
cargo build --all-targets
export PLAYWRIGHT=$(ls -d ~/.npm/_npx/*/node_modules/playwright 2>/dev/null | head -1)
for browser in chromium firefox webkit; do
  BACNETIO_TEST_BROWSER=$browser unshare -Urn bash -c 'ip link set lo up; node tests/ui/run.mjs'
done
```

- **Run them in a network of their own** (`unshare -Urn`, Linux): the tests start the simulators on
  `127.0.0.2` and `127.0.0.3`, port 47809. Simulators already running there for the maintainer
  (a demo, see skill try-it) would answer too and break counts. Inside `unshare` only `lo` exists,
  so nothing outside is touched. `cargo test` the same way when simulators run:
  `unshare -Urn bash -c 'ip link set lo up; cargo test'`.

- Without a Playwright in the npm cache: `npm i -D playwright && npx playwright install chromium firefox webkit`.
- One test only: `node tests/ui/run.mjs <part of its name or file>`.
- Two runs at once (another session in the folder): `BACNETIO_TEST_PORT=18600` for the second.
- A failure leaves a screenshot in the temp folder: `bacnetio-ui-<file>.png`. Look at it with the
  Read tool before guessing.
- Report each part as passed or failed with the failing names; never "done" with a red part.

## Screenshots of the interface

1. Start the simulators and a bacnetio with its own data folder and port in a network of their
   own, so a running one is not disturbed:
   ```sh
   S=<scratchpad>; cargo build --all-targets
   unshare -Urn bash -c "ip link set lo up
     target/debug/examples/simulated_plant 127.0.0.2 47809 &
     target/debug/examples/simulated_controller 127.0.0.3 47809 &
     target/debug/examples/simulated_commands 127.0.0.4 47809 &
     target/debug/bacnetio --no-browser --web-port 18700 --data-dir $S/shots-data --test-loopback &
     sleep 2; node $S/shots.mjs; kill %1 %2 %3 %4"
   ```
   The Playwright script (`shots.mjs`, in the scratchpad) runs inside, as the browser must reach
   the server in that network. A screen with a plugin needs its built file, which is kept
   apart from the repo.
2. In Playwright, set the search settings before loading (subnet `127.0.0.0/29`, port `47809`,
   adapter `127.0.0.1`), as `tests/ui/helpers.mjs` (`open`) does, then search and open a device.
3. Take each screen at 390 × 844 (phone), 1000 and 1440 wide, in light and dark
   (`localStorage bacnetio.theme`), and in German (`bacnetio.language`).
4. Save the pictures in the scratchpad, never in the repo, and look at each with the Read tool:
   nothing cut off, no sideways scrolling, the next step visible, colours from `web/css/tokens.css`.
5. Stop the processes you started (by PID, not `pkill -f`, which can hit other sessions).

## Rules

- Do not change the tests to make them pass; fix the cause, or say why the test is wrong.
- A test that fails once and passes on the same commit elsewhere: read the log before calling it
  flaky. Known: WebKit "internal error" on `page.reload` (avoid reloads; load a new address
  instead), a list redrawn between finding and clicking a row (wait for the box, ask again).
- Say what could not be checked: real devices, real networks, Windows, Android.
