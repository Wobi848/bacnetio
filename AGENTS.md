# Working on bacnetio

BACnet browser. Concept: [docs/concept.md](docs/concept.md), the explorer in
[docs/concept-explorer.md](docs/concept-explorer.md). Everything there is a proposal and open to
change.

Talk to the person you work for in their language. Everything in the repo and on GitHub is in
English: code, comments, docs, commit messages, branches, PRs, issues, CLI commands and flags. The
UI takes the system's language (English, German, French, Italian or Spanish).

Local notes that only concern one person's setup go in `CLAUDE.local.md` or `AGENTS.local.md`;
both are ignored by Git.

## Skills
Recurring tasks have project skills in `.claude/skills/`: `browser-tests`, `review`, `field-data`,
`concept`, `new-text`, `simulator`, `pull-request`, `api-route`, `plugin`, and `builds` (only on
the maintainer's explicit request). Keep them true when the commands, paths or rules they name
change.

## Git and GitHub
- Commits, branches, PRs, issues, tags, release notes, file names and identifiers in English.
- No AI attribution: no `Co-Authored-By`, no "Generated with".
- One short branch per topic, named after the topic. Never commit directly to `main`; changes reach
  it through a pull request that the maintainer merges, with green CI.
- Stage your own files by name, never `git add -A`. Someone else may have changes in the same
  folder.
- Separate commits per concern; the subject says what changes.

## Releases and publishing
- Tags, releases, npm and Windows builds only on the maintainer's explicit request.
- No secrets, real site addresses or personal details in the repo, its history, issues or pull
  requests. Android signing keys never enter the repo.

## Before saying "done"
- Run everything CI runs, plus the browser tests: `cargo fmt --check`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `node --test tests/web/*.test.js tests/plugins/*.test.js`
  and the browser tests in `tests/ui/` (see its README) in Chromium, Firefox and WebKit.
- Say what could not be checked (real devices, real networks, other browsers).
- Never say "done" while known points are open. Name them and work through them; do not stop at
  "the next step would be".
- Bigger changes: concept in `docs/` first, then code.
- User-facing changes go in `CHANGELOG.md` under "Unreleased".
- Docs change in the same pull request as the code: what a user sees in `README.md` (Usage), the
  web API in `docs/api.md`, the plugin API in `docs/plugins.md`, a concept's state in its
  `docs/concept-*.md`. `tests/docs.rs` fails when a route of the API or a simulator in `examples/`
  is not described.

## Never lose a function
- Before replacing or removing something, list every function of the old version and show each one
  in the new version, preferably with a test. Old tests still passing is not enough. The list is
  `docs/concept.md` section 3.3.

## Operation
- This is a work tool: self-explanatory, clean, every function reachable.
- Buttons look like buttons and name their task: word before symbol, no borderless text buttons.
  Clarity before density: rather two lines than one word less.
- Never weaken a control unasked (paler, borderless, symbol only).
- Check screens with screenshots, at phone width too. The colours: one family of cool blue-greys
  with the icon's blue as the only accent (`web/css/tokens.css`); green, orange and red only for
  states.

## Languages
- New texts need every existing language; a test fails otherwise.
- A new language enters the menu only after review by someone from the trade.

## Real plants
- Never write to a real BACnet device unasked, not even to test. Tests run against the simulators
  in `examples/`.
- Every write from the UI stays behind the session unlock, with a clear confirmation and a log entry.
