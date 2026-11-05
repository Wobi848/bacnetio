---
name: plugin
description: "Write or change a bacnetio plugin (one *.bacnetio-plugin.html file, plugin API 1) such as the KBOB check, or change the plugin API itself. Use for checks, reports or views that are installed apart from bacnetio, or when the maintainer asks for an extension, an add-on or \"ein Plugin\"."
---

# bacnetio plugins

Read first: `docs/plugins.md` (for authors) and `docs/concept-plugins.md` (why it is built so). They
arrive with the plugin system; where they are missing, the plugin system is not merged yet.

## A plugin

- **One HTML file** `<id>.bacnetio-plugin.html` with its script, style and data inside; it loads
  nothing from elsewhere.
- Its description as the first `<script type="application/json" id="bacnetio-plugin">` with
  `id`, `name`, `version`, `api: 1`, optional `description`, within the limits in `docs/plugins.md`.
- It reads what bacnetio reads and marks devices and objects through the plugin API. It cannot
  write to the plant, reach bacnetio's server or the user's files; do not look for ways around
  that, and report any you find as a security bug.
- Its texts in all five languages, like bacnetio's (skill new-text).
- Tests: logic in `tests/plugins/*.test.js`, the installed plugin in the browser tests
  (`tests/ui/34-plugins.test.mjs`, `36-kbob.test.mjs`), against a simulator with matching data
  (`examples/simulated_kbob.rs`; skill simulator).

## Changing the plugin API

A plugin API change is a concept change (skill concept): existing plugins with `api: 1` must keep
working, new abilities get a new field or a new API version, and `docs/plugins.md` changes in the
same pull request. Anything that would let a plugin write to a device needs the maintainer's
approval first.
