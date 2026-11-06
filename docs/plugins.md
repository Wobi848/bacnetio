# Writing a bacnetio plugin

A plugin adds a page to bacnetio: a check, a report, a view of its own. It reads what bacnetio
reads and can mark devices and objects in bacnetio's lists. It cannot write to the plant, reach
bacnetio's server or the user's files. Why it is built this way: `concept-plugins.md`.

## The file

A plugin is **one HTML file** named `something.bacnetio-plugin.html`, with its script, style and
data inside: the page may not load anything from elsewhere. It carries its description exactly
like this, as the first such script in the file:

```html
<script type="application/json" id="bacnetio-plugin">
{ "id": "check", "name": "Site check", "version": "1.0.0", "api": 1,
  "description": "Checks the devices of a site against the owner's rules" }
</script>
```

| Field | Rule |
|---|---|
| `id` | up to 40 small letters, digits and dashes; a second plugin with the same id replaces it |
| `name` | 1 to 60 characters, shown in the sidebar and on marks |
| `version` | 1 to 20 characters, shown in the settings |
| `api` | `1`, the plugin API below |
| `description` | up to 300 characters, optional |

The user installs it under *Settings → Extensions → Install extension …*. Once on, it has an
entry in the sidebar, and its page fills the work area. The page keeps running while the user is
elsewhere in bacnetio, until the plugin is switched off.

## Asking bacnetio

The page runs in a sandboxed frame and talks to bacnetio by messages. Copy this into the plugin:

```js
let next = 0;
const waiting = new Map();
addEventListener("message", (event) => {
  if (event.source !== parent) return;
  const { call, result, error } = event.data ?? {};
  const asked = waiting.get(call);
  if (!asked) return;
  waiting.delete(call);
  if (error) asked.reject(new Error(error));
  else asked.resolve(result);
});
const ask = (method, ...args) => new Promise((resolve, reject) => {
  const call = ++next;
  waiting.set(call, { resolve, reject });
  parent.postMessage({ call, method, args }, "*");
});
```

Then, for example, `const devices = await ask("devices");`.

## Plugin API, version 1

| Question | Answer |
|---|---|
| `ask("context")` | `{ plugin, version, language, dark, tokens, device }`: the interface language (`en`, `de`, …), whether the dark look is on, bacnetio's colours as CSS values (`tokens["--accent"]`, …), the device open in the workspace or `null` |
| `ask("devices")` | the devices of the last search or the open site: `instance`, `name`, `address`, `network`, `mac`, `vendor`, `model` |
| `ask("objects", device)` | reads the device's object list with names and values, as the point list does: `type`, `instance`, `name`, `description`, `value`, `units`, `status`, `state_texts`, `error` |
| `ask("properties", device, type, instance)` | all properties of one object, as the details show them: `property` ("present-value"), `value` in words, `error`, `data_type`, and `items` for lists |
| `ask("load")` | the plugin's own store, or `null` |
| `ask("save", data)` | keeps `data` (JSON, up to 1 MB) in the plugin's store, in bacnetio's data folder |
| `ask("download", name, text)` | hands the user a file, such as a report as CSV |
| `ask("mark", marks)` | replaces the plugin's marks; answers how many it took (see below) |
| `ask("priorities", names)` | names the priorities, such as a standard's use of them: `{ "7": "Manual on site", "16": "Automatic mode" }`, keys 1 to 16, up to 24 characters each, in the interface's language. bacnetio shows them under the numbers of the priority grid and in the choice of priority when writing, before BACnet's own names; replaces the plugin's names given before. Answers how many it took. bacnetio before 0.1.3 answers `unknown question` |

Reading goes through bacnetio's own queue, so a slow network (MS/TP behind a router) is not
flooded: ask one device after the other.

bacnetio also tells the plugin when something changed, as a message without `call`:
`{ event: "devices" }` when the devices to ask for change (a search found more, another site was
opened). Ask `devices` again then. The listener above passes over it until the plugin looks for it:

```js
addEventListener("message", (event) => {
  if (event.source === parent && event.data?.event === "devices") showDevices();
});
```

A question bacnetio does not answer fails with `unknown question …`. There is no question that
writes to the plant, and there will not be one in version 1.

### Marks

```js
await ask("mark", [
  { device: 1001, text: "Protocol revision 14 or newer required" },
  { device: 1001, type: 0, instance: 2, text: "No unit" },
]);
```

A mark without `type` and `instance` belongs to the device. bacnetio shows a small badge beside
the device in the device list and the tree, and beside the object in the point list; the text
shows on hover. The badge says the plugin's name, or `label` when the mark gives one (up to 16
characters, such as `"Check ✓"`); `state` `"ok"` makes it green, `"bad"` red, none leaves it
neutral. Each call replaces all marks of the plugin; `[]` clears
them. Switching the plugin off or removing it takes its marks away.

### Looking like bacnetio

Set the page's colours from `context.tokens`, for example:

```js
const { tokens } = await ask("context");
for (const [name, value] of Object.entries(tokens)) document.documentElement.style.setProperty(name, value);
```

and use `var(--text)`, `var(--bg)`, `var(--accent)` in the style.

## What a plugin cannot do

- write to devices, give commands, release values;
- send requests anywhere (`fetch` and the like are blocked by the page's policy);
- read bacnetio's page, its settings or other plugins' stores;
- open windows or navigate bacnetio.
