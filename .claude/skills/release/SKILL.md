---
name: release
description: "Publish a bacnetio release (version, changelog date, tag, release notes, checking the files) only when the maintainer explicitly asks for that release in the current conversation (\"mach 0.1.3\", \"neues Release\", \"jo machen wir so\" to the question whether to release). A release is public and cannot be taken back."
disable-model-invocation: true
---

# A bacnetio release

Only on the maintainer's explicit word in this conversation (AGENTS.md). A `v*` tag builds and
publishes for everyone; ask first when anything is unclear.

## Before

- Every pull request meant for it is merged (skill pull-request), `main` is green.
- Read `CHANGELOG.md` "Unreleased" against the merged pull requests: every user-facing change is
  there, in English, under Changed or Fixed (Added for new features).
- Bump by semver: patch for fixes only, minor for new functions, major when something breaks.

## Steps

1. Branch `release-vX.Y.Z` from `origin/main`:
   - `Cargo.toml`: `version = "X.Y.Z"`; `cargo build` updates `Cargo.lock`. The Android version
     follows from it (`android/app/build.gradle.kts`).
   - `CHANGELOG.md`: `## Unreleased` becomes `## vX.Y.Z (DD.MM.YYYY)`, today's date.
   - Commit "Prepare release vX.Y.Z", pull request with the same title, wait for green CI, merge on
     the maintainer's OK.
2. Tag on the merged `main`:
   ```sh
   git fetch && git switch main && git merge --ff-only origin/main
   grep -m1 '^version' Cargo.toml          # must say X.Y.Z
   git tag vX.Y.Z && git push origin vX.Y.Z
   ```
3. Wait for the run of the tag (`gh run list --branch vX.Y.Z`). The `release` job needs `build`,
   `android` and `plugins`; a red browser test there does not stop it but must be green again
   (read the log, `gh run rerun <id> --failed` only for a known browser flake).
4. Release notes from the changelog section:
   ```sh
   awk '/^## vX.Y.Z/{on=1;next} /^## v/{on=0} on' CHANGELOG.md > <scratchpad>/notes.md
   gh release edit vX.Y.Z --notes-file <scratchpad>/notes.md
   ```
5. Check the files: `gh release view vX.Y.Z --json assets -q '.assets[].name'`: the Windows exe,
   the Linux program and the Android APK.

## After

Tell the maintainer in German: the link, what is new in three lines, how to update (close
bacnetio, start the new exe; on Android the GitHub APK installs over a GitHub APK, a test APK signed
with another key needs uninstalling first), and what was not tested on real devices.
