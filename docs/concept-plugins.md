# Concept: plugins

The maintainer wants extensions such as the KBOB check (`docs/concept-kbob-check.md`) to be
installed apart from bacnetio, not built in (5 November 2026: "ich will plugin system"). This
concept says what a plugin is, what it may do and how it is installed. Built on 5 November
2026 (`src/plugin.rs`, `src/server/plugins.rs`, `web/plugin*.js`); for authors: `docs/plugins.md`.

## Aims

- bacnetio itself stays as it is; whoever needs no extension sees none.
- A plugin is installed, switched off and removed in the interface, without a new bacnetio.
- A plugin can never write to the plant or touch the user's files. A plugin from someone else is
  as safe to try as a web page.
- One way for Windows, Linux and the Android app.

## What a plugin is

**One file**, `name.bacnetio-plugin.html`: an HTML page with its script, style and data inside.
At the top it carries its description as JSON:

```html
<script type="application/json" id="bacnetio-plugin">
{ "id": "kbob", "name": "KBOB check", "version": "1.0.0", "api": 1,
  "description": "Checks devices against the KBOB recommendation BACnet Anwendung V2.0" }
</script>
```

One file needs no unpacking (no zip library) and is easy to pass on by mail or USB stick.

## Install, switch, remove

**Settings → Extensions:**

- **Install…** opens a file choice. The server checks the description (id, version, `api` it knows),
  then keeps the file in the data folder under `plugins/`. A second install of the same id replaces
  it after asking, showing the old and new version.
- Each plugin shows with name, version and description, an on/off switch and **Remove**.
- On, it gets an entry in the sidebar below the areas, with its name.

Before the install the page says plainly that a plugin can read everything bacnetio reads, but
not write.

## How it runs

The plugin page runs in a **sandboxed frame** inside bacnetio's content area: scripts allowed,
no access to bacnetio's page, its storage or its server. It speaks to bacnetio only through
messages (`postMessage`), and bacnetio answers only the questions of the plugin API. So a plugin
cannot call the write endpoints at all, not even by mistake.

## Plugin API, version 1

Questions a plugin may ask; each one goes through the same server functions as bacnetio's own
pages, and so through the same queue (`src/precedence.rs`):

| Question | Answer |
|---|---|
| `context` | language, theme, the open site and device, bacnetio's version |
| `devices` | the devices of the last search or the open site |
| `objects(device)` | the device's object list with type, instance and name |
| `properties(device, type, instance)` | all properties of one object, as the details read them |
| `load()` / `save(data)` | a small store of the plugin's own, kept in the data folder beside the site |
| `download(name, text)` | hand the user a file, such as a report as CSV |
| `mark(marks)` | short marks on devices and objects that bacnetio shows in its own lists and details (see below) |
| `priorities(names)` | names for the 16 priorities, shown under each number in the priority grid and in the choice when writing, before BACnet's own names (KBOB: table 10) |

**Marks** let a plugin point at what it found without a page of its own: a list of
`{device, type?, instance?, text}`, shown as a small badge with the plugin's name in the device
list, the tree, the point list and the details, the text on hover and in the details. A filter
*Marked by …* shows only marked rows. Marks change only what bacnetio shows, never the plant; a
plugin switched off or removed takes its marks with it.

**Priority names** let a standard name what each priority is for. bacnetio shows BACnet's own
names (1 Manual Life Safety, 2 Automatic Life Safety, 5 Critical Equipment Control, 6 Minimum
On/Off, 8 Manual Operator); KBOB gives every priority a use or calls it free (table 10). A plugin
gives `{ "7": "Manual on site", … }`, up to 24 characters each, in the interface's language; the
details say which plugin named it. Switched off or removed, the names go with it.

bacnetio tells the plugin of changes as messages without `call` (`{ event: "devices" }`), which a
plugin of version 1 passes over.

No writing, no commands to devices, no network beyond bacnetio. A question added to version 1
changes nothing for a plugin that does not ask it; a plugin that asks it of an older bacnetio gets
`unknown question`. What would break version 1 plugins comes as API version 2.

The plugin gets bacnetio's colours and fonts as CSS variables, so it looks like the rest.

## Where plugins come from

- Plugins of this project live in the repo under `plugins/`, each with its own tests, and are
  built into their single file for every release (`kbob.bacnetio-plugin.html` beside the exe).
- Anyone can write one; the API is described in `docs/plugins.md`.
- No plugin store in bacnetio for now.

## Android

The app keeps the same data folder and the same WebView: a plugin is installed through the
file choice of Android, and runs in the same sandboxed frame.

## Never lose a function

Nothing in bacnetio moves into a plugin; the KBOB check is new.

## Work

1. Server: the plugin folder, install (check, keep), list, switch, remove, serving a plugin file,
   the plugin store.
2. Web: *Extensions* in the settings, the sidebar entry, the frame and the message bridge with
   API version 1.
3. A small test plugin in the tests: installed, asks each question, cannot reach a write.
4. `docs/plugins.md` for plugin authors.
5. Then the KBOB check as the first plugin.

## Decided

5 November 2026, with the maintainer: one file per plugin, installed through *Settings →
Extensions*; read-only in API version 1; plugins mark what they find in bacnetio's lists.
