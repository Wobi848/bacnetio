---
name: pull-request
description: "Finish a bacnetio topic and hand it over as a pull request: own branch, commits staged by name, CHANGELOG, docs and function list in the same PR, full checks, PR text without AI attribution; then watch CI, read failing logs and merge on the maintainer's OK. Use whenever a bacnetio change is done and should reach main, when asked to commit, push, open, update, describe or merge a PR, or when CI is red."
---

# From a topic to a pull request

Changes reach `main` only through a pull request, with green CI, merged on the maintainer's explicit
OK ("merge", "jo machen wir so" to the question whether to merge). Never commit to `main`, never
merge without that OK, never tag or release (skill release).

## Branch and commits

- One short branch per topic, named after it (`release-check`, `sidebar-head`).
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

## CI

- Wait for it to finish: `gh pr checks N` until nothing is pending (a loop with `timeout 30 tail -f
  /dev/null` as the pause; a plain `sleep` may be blocked).
- A red job: read its log before anything else:
  `gh api repos/Wobi848/bacnetio/actions/jobs/<job id>/logs | grep -A6 "✗"` (the job id is in
  `gh pr checks N`). Fix the cause. Only a known flake of the browser itself (WebKit "internal
  error") is run again: `gh run rerun <run id> --failed`, and then made robust in the test.
- Two runs per commit (push and pull_request): both must be green.

## Merging, on the maintainer's OK

```sh
gh pr merge N --squash --delete-branch -A <the maintainer's no-reply address> --subject "<subject> (#N)"
```

The `-A` keeps the maintainer as author; the address is in `CLAUDE.local.md`. Branches stacked on the merged one are
then rebased onto `origin/main` (`git rebase origin/main`, or `--onto origin/main <old base>`);
`CHANGELOG.md` usually conflicts under "Unreleased": keep both sides, each line under its heading.
Force-push only your own branch. Merge in the order the branches build on each other.

No real site names, addresses or personal details in commits, PRs or issues: the repo is public.
