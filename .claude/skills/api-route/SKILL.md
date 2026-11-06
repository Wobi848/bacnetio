---
name: api-route
description: "Add or change a route of bacnetio's local web API in src/server/, with the Host/Origin guard, input checks at the server, a journal entry when it changes a device, docs in docs/api.md and tests. Use whenever the interface needs new data or a new action from the server, or a handler in src/server/ changes."
---

# A route of the web API

## Where

- Routes in `src/server/mod.rs` (`router`); handlers in the module of their area (`devices.rs`,
  `writing.rs`, `commands.rs`, `reading.rs`, `site.rs`, `plugins.rs` …). The logic lives outside
  `server/` (layers: the server knows HTTP, the logic knows BACnet). Split a file over ~400 lines.
- Every route passes the `Host`/`Origin` guard (`src/server/guard.rs`) through the router layer;
  never add a route beside it.

## Rules

- **Check input at the server**, not only in the page: ranges, finite numbers (`NaN`, `inf`), empty
  fields, IDs in range; answer `400` in words.
- **Errors** as `ApiError` with the status from `docs/api.md` (400 wrong request, 403 foreign page,
  404 nothing of that name, 409 not possible now, 502 the device did not); English text, and a pattern plus texts in every
  language when the interface shows it (skill new-text).
- **Anything that changes a device** goes through `writing::journaled` (or records a log entry), so
  it lands in the log refused or not, is covered by "held by you" and the leaving check when it
  stays in the plant, and sits behind the session unlock with a confirmation in the page. No
  passwords in the journal.
- **Stored data and API fields** stay compatible: add fields, do not rename or remove them.

## Docs and tests

- Describe the route in `docs/api.md`: method, path, request, answer, errors. `tests/docs.rs`
  fails otherwise.
- A test for the behaviour: `tests/api.rs` for answers without a network, a Rust test against a
  simulator (`tests/common`) for BACnet, a browser test for what the user does.
- `CHANGELOG.md` when a user notices it.
