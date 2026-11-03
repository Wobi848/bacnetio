## What and why

<!-- What changes for the user, and why. Link the issue: "Closes #123". -->

## How it was checked

<!-- Tests added or changed; what you tried in the interface; on which devices or simulators. -->

- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [ ] `node --test tests/web/*.test.js` and the browser tests (`tests/ui/README.md`)
- [ ] New logic has a test that fails without the change
- [ ] New interface texts are in every language in `web/lang/`
- [ ] `CHANGELOG.md` updated under *Unreleased* (user-facing changes)
- [ ] Nothing removed without showing where it lives on
- [ ] No secrets, real site addresses or customer data

## Not checked

<!-- What could not be tested, for example a real device, a router to MS/TP, Windows. -->
