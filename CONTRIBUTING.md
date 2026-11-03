# Contributing to bacnetio

Thank you for helping. bacnetio is a work tool for people on building automation sites, so two
things come before everything else: it must never do anything to a plant the user did not ask
for, and it must say in plain words what is going on.

## Before you start

- For a bug, open an issue with the template; for anything bigger than a fix, open an issue first
  and describe what you want to change. Larger changes start as a concept in `docs/`
  (see the `concept-*.md` files there), agreed before the code.
- Everything in the repository is in English: code, comments, docs, commits, branches, issues and
  pull requests.

## Set up

```sh
cargo build --all-targets
cargo run                     # web server on http://127.0.0.1:8081/, opens the browser
```

The simulated devices in `examples/` stand in for a plant. **Never test against a real BACnet
device you are not responsible for, and never write to one to try something out.**

## Checks

Run what CI runs, and the browser tests, before you open a pull request:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --test tests/web/*.test.js
node tests/ui/run.mjs         # see tests/ui/README.md; Chromium, Firefox and WebKit
```

The integration and browser tests talk real UDP on loopback addresses. On Linux they can run in a
network namespace of their own, so a BACnet tool running at the same time does not interfere:
`unshare -Urn bash -c 'ip link set lo up; cargo test'`.

## Rules for changes

- **One topic per branch and pull request**, named after the topic (`column-resize`). Separate
  commits per concern; the subject says what changes.
- **Tests with new logic.** They check behaviour, not implementation details. A fix comes with a
  test that fails without it.
- **Never lose a function.** Before replacing or removing something, list what the old version did
  and show where each part lives on (`docs/concept.md` section 3.3).
- **Writes stay safe.** Every write to a device stays behind the session unlock, with a clear
  confirmation and an entry in the write log.
- **Every language.** New interface texts go into every file in `web/lang/`; a test fails
  otherwise. A new language enters the menu only after review by someone from the trade
  (`docs/concept-languages.md`).
- **Buttons say what they do.** A word before a symbol; no borderless text buttons; clarity before
  density.
- **Changelog.** User-facing changes go in `CHANGELOG.md` under *Unreleased*.
- **No secrets and no plant data:** no passwords, keys, real site addresses or customer names in
  code, tests, screenshots or commits. Use example addresses (`192.168.10.x`, `127.0.0.x`).

## Pull requests

Fill in the template. A pull request is merged by the maintainer once CI is green and the review is
done. Releases and tags are made by the maintainer only.

## Working with AI assistants

`AGENTS.md` holds the same rules for AI coding assistants. Whoever uses one is responsible for what
it writes, as for their own code.
