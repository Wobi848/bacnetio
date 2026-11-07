# Browser tests of the interface

The tests start the two simulated devices from `examples/` on loopback addresses (`127.0.0.2` and
`127.0.0.3`, port 47809), then per test a fresh bacnetio with an empty data folder and the hidden
`--test-loopback` switch, operate the interface like a user and check the result. No real network
and no administrator rights are needed. On Linux they also start `simulated_mstp`, an MS/TP device on a
pseudo-terminal, and hand its name to bacnetio in `BACNETIO_TEST_SERIAL`; elsewhere the MS/TP test
has nothing to listen to and passes without checking.

```sh
cargo build --all-targets
npm i -D playwright && npx playwright install chromium firefox webkit   # once
node tests/ui/run.mjs            # all
node tests/ui/run.mjs layout     # only tests with "layout" in their name or file name
```

Playwright already installed elsewhere: `PLAYWRIGHT=/path/to/node_modules/playwright`.
Firefox or the Safari engine: `BACNETIO_TEST_BROWSER=firefox` or `webkit`.
Several runs at the same time: `BACNETIO_TEST_PORT=<first web port>` (default 18400).
After a failure there is a screenshot in the temp folder (`bacnetio-ui-<file>.png`).

`27-layout.test.mjs` checks every page in every language, at phone, two medium and a wide width, in
light and dark, before the first search and with a controller of 170+ points and long names. It
fails on controls outside the window, cut off buttons, states or chips, and sideways scrolling.

Loopback addresses other than 127.0.0.1 exist on Linux out of the box; on macOS add them first
(`sudo ifconfig lo0 alias 127.0.0.2` and `127.0.0.3`). Windows runs the tests in WSL.
