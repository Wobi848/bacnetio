---
name: pull-request
description: "Finish a bacnetio topic and hand it over as a pull request: own branch, commits staged by name, CHANGELOG, docs and function list in the same PR, full checks, PR text without AI attribution. Use whenever a bacnetio change is done and should reach main, or when asked to commit, push, open, update or describe a PR — main is only reached through the maintainer's merge."
---

# From a topic to a pull request

Changes reach `main` only through a pull request that the maintainer merges, with green CI. Never
commit to `main`, never merge yourself, never tag or release.

## Branch and commits

- One short branch per topic, named after it (`release-check`, `kbob-plugin`).
- Stage your own files by name, never `git add -A`: another session may have changes in the folder.
  Check `git status` first and leave files you did not change alone.
- Separate commits per concern; the subject says what changes, in English. No AI attribution: no
  `Co-Authored-By`, no "Generated with".

## In the same pull request as the code

- `CHANGELOG.md` under "Unreleased": what a user notices.
- `README.md` (Usage) for what a user sees; `docs/api.md` for routes (`tests/docs.rs` checks every
  route and simulator); `docs/plugins.md` for the plugin API; the concept's status line.
- `docs/concept.md` section 3.3 when a function is added or moves, with its test.
- Texts in all five languages (skill new-text).

## Before opening it

Run the full check set (skill browser-tests) and say in the PR which parts ran and what could not
be checked (real devices, real networks, Windows, Android).

## The PR text

In English, short: what changes for the user, why, how it was tested, what is open. Mention
behaviour changes and anything that writes to a plant. No AI attribution. Then tell the maintainer
in German that it waits for the merge and what to look at.

No real site names, addresses or personal details in commits, PRs or issues: the repo is public.
