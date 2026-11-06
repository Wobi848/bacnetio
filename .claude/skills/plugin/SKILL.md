---
name: plugin
description: "Write or change a bacnetio plugin (one *.bacnetio-plugin.html file, plugin API 1) such as a check against a standard, or change the plugin API itself. Use for checks, reports or views that are installed apart from bacnetio, or when the maintainer asks for an extension, an add-on or \"ein Plugin\"."
---

# bacnetio plugins

Read first: `docs/plugins.md` (for authors) and `docs/concept-plugins.md` (why it is built so). They
arrive with the plugin system; where they are missing, the plugin system is not merged yet.

## A plugin

- **One HTML file** `<id>.bacnetio-plugin.html` with its script, style and data inside; it loads
  nothing from elsewhere.
- Plugins are kept and released apart from this repo, which holds none (6 November 2026); only the
  plugin API and its test plugin (`tests/ui/fixtures/test.bacnetio-plugin.html`) live here. Keep a
  plugin's logic in modules without I/O so `node --test` can test it, and build the one file from
  them.
- **The build puts every file of a plugin into one scope** (imports go, `export` goes): two files
  declaring the same top-level name break the plugin, and it shows nothing.
  Parse every built file in a test; give names that could clash an own prefix.
- Its description as the first `<script type="application/json" id="bacnetio-plugin">` with
  `id`, `name`, `version`, `api: 1`, optional `description`, within the limits in `docs/plugins.md`.
- It reads what bacnetio reads and marks devices and objects through the plugin API. It cannot
  write to the plant, reach bacnetio's server or the user's files; do not look for ways around
  that, and report any you find as a security bug.
- Its texts in the plugin (`texts.js`), in the interface's five languages like bacnetio's; French,
  Italian and Spanish are drafts that bacnetio shows only for review (`?lang=`) until someone from
  the trade has read them. A test checks that every language has every text with the same
  placeholders.
- Rules taken from a standard or recommendation: skill standards.
- Tests here: every question of the API through the test plugin in `tests/ui/34-plugins.test.mjs`;
  a new question gets a line in the test plugin and an assertion there.

## Changing the plugin API

A plugin API change is a concept change (skill concept): existing plugins with `api: 1` must keep
working, new abilities get a new field or a new API version, and `docs/plugins.md` changes in the
same pull request. Anything that would let a plugin write to a device needs the maintainer's
approval first.
