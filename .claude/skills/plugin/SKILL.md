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
- The project's plugins live in `plugins/<id>/`: `page.html` with the description and
  `<script src="x.js"></script>` lines, the logic as modules (`check.js` without I/O, testable with
  `node --test`). `node plugins/build.mjs [folder]` puts each into its one file (default
  `target/plugins/`); a release carries it beside the programs.
- **The build puts every file of a plugin into one scope** (imports go, `export` goes): two files
  declaring the same top-level name break the plugin, and it shows nothing.
  `tests/plugins/build.test.js` parses every built file; give names that could clash an own prefix.
- Its description as the first `<script type="application/json" id="bacnetio-plugin">` with
  `id`, `name`, `version`, `api: 1`, optional `description`, within the limits in `docs/plugins.md`.
- It reads what bacnetio reads and marks devices and objects through the plugin API. It cannot
  write to the plant, reach bacnetio's server or the user's files; do not look for ways around
  that, and report any you find as a security bug.
- Its texts in the plugin (`texts.js`), in the interface's five languages like bacnetio's; French,
  Italian and Spanish are drafts that bacnetio shows only for review (`?lang=`) until someone from
  the trade has read them. `tests/plugins/kbob.test.js` checks that every language has every text
  with the same placeholders; a new plugin gets the same test.
- Rules taken from a standard or recommendation (KBOB): skill standards.
- Tests: logic in `tests/plugins/*.test.js`, the installed plugin in the browser tests
  (`tests/ui/34-plugins.test.mjs`, `36-kbob.test.mjs`), against a simulator with matching data
  (`examples/simulated_kbob.rs`; skill simulator).

## Changing the plugin API

A plugin API change is a concept change (skill concept): existing plugins with `api: 1` must keep
working, new abilities get a new field or a new API version, and `docs/plugins.md` changes in the
same pull request. Anything that would let a plugin write to a device needs the maintainer's
approval first.
