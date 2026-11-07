# bacnetio HTTP API

`bacnetio` without a command starts a local web server on `http://127.0.0.1:8081/` (next free
port if taken, `--web-port` to choose) and opens the browser. The server serves the user interface
from `web/` and this JSON API under `/api/`.

- Only `127.0.0.1` is served, because the API can write to the plant. There is no CORS: the UI
  has to be served by bacnetio itself (put it into `web/`, `index.html` is the start page).
  Every request must name this server as `Host` (`127.0.0.1:PORT` or `localhost:PORT`), and an
  `Origin`, when sent, must be this server too; otherwise `403`. This keeps pages elsewhere out,
  also through DNS rebinding.
- In debug builds (`cargo run`) files in `web/` are read from disk on every request, so editing
  the UI needs no rebuild. Release builds embed them.
- Errors come as `{"error": "text in plain words"}` with status
  - `400`: the request is wrong (bad value, priority out of range, unknown adapter)
  - `403`: the request did not come from a page of this server
  - `409`: not possible now (no scan yet, a scan is already running)
  - `502`: the network or a device did not do what was asked
- Texts (`value`, `status`, `object`, notes, errors) are English and ready to show. Notes also
  carry a `code` to translate by.

## Version

`GET /api/info`: `{ "version": "0.1.0" }`

`GET /api/update` asks GitHub for the newest release (docs/concept-update-check.md):
`{ "current": "0.1.0", "latest": { "version": "0.2.0", "url": "https://github.com/…/releases/tag/v0.2.0" }, "newer": true }`.
`latest` is `null` while nothing is published. Without an answer from GitHub: 502 with `error`.

## Adapters

`GET /api/adapters`

```json
[
  { "name": "Ethernet", "ip": "192.168.10.5", "prefix_len": 24, "kind": "cable", "default": true },
  { "name": "WLAN", "ip": "10.0.0.7", "prefix_len": 24, "kind": "wireless", "default": false }
]
```

`kind` is `cable`, `wireless` or `virtual` (VPN, VM, container). `default` marks the adapter
bacnetio would pick. `description` is the system's adapter description where it has one apart
from the name (Windows: `"VirtualBox Host-Only Ethernet Adapter"` behind `"Ethernet 2"`), else
`null`.


## MS/TP over a serial port

Windows and Linux only; the Android app has no serial port (`docs/concept-mstp.md`).

`GET /api/serial-ports`: the serial ports of this computer, USB adapters first. `description` is
the adapter's own name, when it gives one.

```json
[
  { "name": "/dev/ttyACM0", "description": "USB Single Serial", "usb": true },
  { "name": "/dev/ttyS0", "description": null, "usb": false }
]
```

`POST /api/mstp/listen` `{ "port": "/dev/ttyACM0", "baud": 38400 }`: listens to the trunk on that
port without joining it; nothing is sent. Without `baud`, it listens about two seconds at each
speed (38400, 76800, 19200, 9600, 115200) until one gives clean frames. The answer:

```json
{
  "baud": 38400,
  "trials": [{ "baud": 38400, "frames": 212, "damaged": 0 }],
  "stations": [5, 6, 7, 8],
  "max_master": 127,
  "lost_tokens": 0,
  "proposed_station": 127
}
```

`baud` is `null` when no speed gave clean frames; `trials` then says what each speed heard
(`damaged`: broken CRC, cut-off frames, octets that make no frame). `stations` sent a frame,
`max_master` is the highest address their polls reach, `proposed_station` the highest free
address up to it. `400` for a port the system does not list or a speed MS/TP does not use, `409`
when the port cannot be opened (in use, no permission: the text says what to do).
## Search for devices

`POST /api/scan/stop` stops a running search (`204`); it ends as done, with the devices found so far.

`POST /api/scan` starts a search and answers `202` at once. All fields are optional:

```json
{ "interface": "192.168.10.5", "port": 47808, "bbmd": "10.1.0.1:47808", "sweep": true }
```

- `interface`: adapter IP; default adapter if missing
- `port`: BACnet UDP port, default 47808
- `bbmd`: register as foreign device at this BBMD to reach other subnets (IP or IP:PORT)
- `subnet`: ask every address of this subnet (e.g. `192.168.10.0/24`) instead of the adapter's own;
  for a plant behind a VPN, which does not pass on broadcasts
- `id_from`, `id_to`: only devices with an ID in this range (Who-Is with range; on large sites
  this keeps the answers from flooding the network)
- `sweep`: also ask every address of the subnet directly (finds devices behind a firewall that
  drops broadcast answers); default `true`

The first search opens the BACnet connection. Later searches reuse it, or replace it when
interface, port or BBMD change. A BBMD that refuses or does not answer gives `502` with the reason.

`GET /api/scan`: poll this (e.g. every 500 ms) while `state` is `running`.

```json
{ "state": "running", "phase": "sweep", "asked": 120, "total": 254, "found": 3, "error": null }
```

- `state`: `idle`, `running`, `done`, `failed`
- `phase` while running: `who_is` (about 3 s), `sweep` (with `asked` of `total`), `details`
  (reading names of the devices found), `objects` (only object lists are still being read). Devices
  come with their name and object list as soon as they are read, in any phase.
- `failed`: `error` says why
- `objects`: how far the object list of the device is read, with the device object itself, as
  identifiers only. `error` says why it could not be read; the search goes on without it. The first
  reading of the device uses this list, reading again asks the device.

When `done`, the result is added:

```json
{
  "state": "done", "found": 1, "error": null, "phase": null, "asked": 0, "total": 0,
  "devices": [
    {
      "instance": 1001, "address": "192.168.10.20", "network": null, "mac": null,
      "name": "AHU-01", "vendor": "Siemens", "model": "PXC4", "found_by": "who_is", "error": null,
      "objects": { "read": 170, "total": 170, "done": true, "error": null }
    }
  ],
  "notes": [{ "code": "broadcast_blocked", "direct": 1, "text": "No device answered the Who-Is broadcast, but 1 answered ..." }],
  "refused": [{ "address": "192.168.10.30", "reason": "..." }]
}
```

Before the first search after a start, the answer is the last finished search from the data folder
(`site.json`), marked with `remembered_ms` (when it was saved, milliseconds since 1970). Each search
is also kept as a site of its own (see below); searching the same site again replaces it.

- `address`: IP, or IP:PORT when not 47808. For a device behind a BACnet router it is the router;
  `network` and `mac` (hex) then say where the device sits.
- `found_by`: `who_is` or `direct` (only found by asking its address)
- `error`: some details (name, vendor, model) could not be read
- `notes`: diagnosis, worth showing prominently when no device was found. Each note has a `code`,
  its values (e.g. `direct`, `addresses`) and `text`, the English sentence. A UI in another
  language translates by `code` and falls back to `text`. Codes: `link_local`, `port_in_use`
  (`reason`), `subnet_too_large` (`prefix_len`, `max_prefix_len`), `no_answer`,
  `no_answer_sweep_disabled`, `no_answer_subnet_too_large`, `none_identified`, `broadcast_blocked`
  (`direct`), `broadcast_not_passed` (`direct`; the same on a VPN or other virtual adapter),
  `some_only_direct` (`direct`, `total`), `duplicate_id` (`instance`, `addresses`),
  `refused` (`addresses`), `silent_networks` (`networks`: a router reaches them, but no device
  there answered)
- `refused`: addresses that speak BACnet but would not tell their device ID
- `routers`: `[{ "address": "192.168.10.5", "networks": [5, 7] }]`, the BACnet routers that answered
  Who-Is-Router-To-Network and the networks they reach

`GET /api/devices.csv` downloads the devices of the last finished scan as CSV (ID, name, vendor,
model, address, network, MAC, found by), for the site documentation.

`POST /api/devices/locate` `{ "address": "192.168.20.15", "instance": 1001 }` opens one device by
its address (IP, or IP:PORT), for when a scan does not reach it, e.g. another subnet without BBMD.
`instance` is needed only if the device refuses to tell its ID. A device behind a BACnet router takes
the router as `address`, plus `network`, `mac` (`12`, or bytes in hex such as `00:1a`) and `instance`. Takes the same optional
`interface`, `port` and `bbmd` as the scan and answers with the device as in the scan list; `502`
with the reason when nothing answers there.

## Points of a device

`GET /api/devices/{instance}/points` reads the object list and name, value, unit and status of
every object. This can take a while on large or slow devices (seconds to a minute); the request
simply waits.

```json
{
  "points": [
    {
      "type": 0, "instance": 1, "object": "Analog input 1",
      "name": "Supply air temp", "description": "AHU-01 supply",
      "value": "21.4 °C", "units": "°C", "status": "", "state_texts": [], "error": null
    },
    {
      "type": 19, "instance": 1, "object": "Multi-state value 1",
      "name": "Operating mode", "description": "",
      "value": "Day", "units": null, "status": "", "state_texts": ["Off", "Day", "Night"], "error": null
    }
  ],
  "notes": [{ "code": "no_read_property_multiple", "text": "The device does not handle ReadPropertyMultiple; values were read one by one." }]
}
```

- `state_texts`: texts of a multi-state object's states, from state 1 on; `value` already shows
  the text. Values from other calls still say `"state 2"`, so the UI looks the text up here.
- `type`: BACnet object type number (0 analog input, 1 analog output, 2 analog value, 3 binary
  input, 4 binary output, 5 binary value, 13/14/19 multi-state input/output/value, ...)
- `status`: set status flags in words (`alarm`, `fault`, `overridden`, `out of service`); empty
  when all is well
- `error`: what could not be read for this object
- `notes` as in the scan, codes: `object_list_entry_by_entry`, `no_read_property_multiple`,
  `stopped_answering` (`unread`)

`POST /api/devices/{instance}/read/stop` stops a reading (`204`). What was read stays, with the note `stopped_by_user`.

`POST /api/devices/{instance}/read` starts reading the device in the background (unless that
already runs) and answers like `GET /api/devices/{instance}/read`, which tells how far it is:

```json
{
  "state": "running", "phase": "values", "listed": 170, "total": 170, "read": 64,
  "points": [ … as above, names and values only for the first 64 … ],
  "notes": [], "error": null,
  "requests": [
    { "at_ms": 0, "what": "ReadPropertyMultiple, max-apdu-length-accepted and segmentation-supported", "took_ms": 38, "failed": null },
    { "at_ms": 41, "what": "ReadProperty object-list[0]", "took_ms": 35, "failed": null }
  ]
}
```

- `state`: `idle`, `running`, `done` or `failed` (`error` says why)
- `phase` while running: `limits` (asking the device for its largest answer and segmentation),
  `list` (object list, `listed` of `total`), `values` (`read` of `total`)
- `requests`: the request log, every request of the reading with its time since the start, its
  duration and why it failed; the last 500 are kept. Request sizes follow the device's limits.

`GET /api/devices/{instance}/points/last` answers the last reading of the device without asking
it again, with empty `notes`. After a restart it answers the points remembered in `site.json`;
`null` if the device was not read since the last search. The UI uses it after a reload.

`GET /api/devices/{instance}/points.ede.csv` downloads the last reading as EDE file (Engineering
Data Exchange, B.I.G.-EU layout 2.2, semicolon, UTF-8) for management systems. `409` if the
points were not read yet.

`?project=` gives the file's project name (default: the device's name). `settable` says what the
object type allows (`Y` for outputs and values, `N` otherwise). `state-text-reference` points into
`GET /api/devices/{instance}/points.ede-states.csv`, the state text file that goes with it.
`supports COV` and the limits stay empty, because reading them would take many more requests.

`POST /api/devices/{instance}/instance` `{ "instance": 1234 }` gives the device a new device ID, logged
like a write. `400` for an ID out of range or one another device of the last search has, `502` with
the reason when the device refuses, as many do. Afterwards the device answers under the new ID only,
so search again.

`POST /api/devices/{instance}/values` `{ "objects": [{ "type": 0, "instance": 1 }] }` reads up to 200
objects again and answers them as in the point list. The UI uses it for what is on screen.

`GET .../objects/{type}/{instance}/value?mode=read` asks the device instead of following its COV
reports. Without it, COV is used where the device offers it.

`GET /api/devices/{instance}/priorities` reads which priority wins on each commandable object, one
request per object: `[{ "type": 1, "instance": 1, "winner": 8 }]`, objects without a set priority left
out. `409` if the points were not read yet.

`GET /api/devices/{instance}/structure` reads the plant view the device carries itself: its
Structured View objects (type 29) with their Subordinate_List and Subordinate_Annotations, one
request per property. `409` if the points were not read yet, because the views are taken from the
last reading's object list.

```json
{
  "views": [
    {
      "type": 29, "instance": 1, "object": "Structured view 1", "name": "AHU-01 plant",
      "subordinates": [
        { "type": 29, "instance": 2, "object": "Structured view 2", "device": null, "annotation": "Air handling", "name": "Air handling" },
        { "type": 0, "instance": 7, "object": "Analog input 7", "device": 1002, "annotation": "Weather station", "name": null }
      ],
      "error": null
    }
  ]
}
```

- `views` is empty when the device has no such objects; the UI then groups by object names.
- `device` is the device that holds the subordinate, or `null` for this device; `name` is the
  object's name from the last reading, and `null` for an object on another device.
- `annotation` is what the view says about the entry, `null` without Subordinate_Annotations.
- `error` on a view says why its subordinate list could not be read; the view keeps its place.
- A view that names another view builds the tree. Views that hold each other are each read once.
- Nothing is written: the tables are read only.

`GET /api/devices/{instance}/bbmd` reads the two tables of a BBMD at the device's address, with
Read-Broadcast-Distribution-Table and Read-Foreign-Device-Table (Annex J): which BBMDs it
distributes broadcasts to, and which devices are registered with it. `409` before the first
search, because the address comes from it. Nothing is ever written to a BBMD.

```json
{
  "is_bbmd": true,
  "bdt": [
    { "address": "10.0.0.1", "mask": "255.255.255.255", "own": true },
    { "address": "10.1.0.1", "mask": "255.255.255.0", "own": false }
  ],
  "fdt": [{ "address": "10.2.0.9:47809", "ttl_s": 300, "seconds_left": 262 }],
  "bdt_note": null,
  "fdt_note": null
}
```

- `is_bbmd`: the device gave its Broadcast Distribution Table, which only a BBMD does.
- `mask` all ones means the broadcast goes to that BBMD itself (two-hop), else to its subnet;
  `own` marks the asked device's own row.
- `ttl_s` is the lifetime the foreign device asked for, `seconds_left` what the BBMD says is left
  of it (grace period included).
- A table that could not be read is `null` and its note says why, as a code with English text:
  `no_bbmd` (it refused, as every node that is no BBMD does), `silent` (no answer at all),
  `failed` (`reason`). A device that is no BBMD is an answer, not an error: the call still gives
  `200`.

`POST /api/devices/{instance}/objects/{type}/{instance}/property` `{ "property_id": 81, "data_type":
"boolean", "value": "true" }` writes a property with a plain value, logged like a write, and answers
the object as read again. `GET .../properties` gives each property's `id`, `data_type` and whether it
is `writable` this way; the device decides whether it takes it. The present value and the object's
identity are not written this way.

`POST /api/devices/{instance}/restart` `{ "kind": "warm", "password": "…" }` (`warm`, `cold`) and
`POST /api/devices/{instance}/communication` `{ "enable": false, "minutes": 10, "password": "…" }` send
those commands, logged; the password is never stored. `POST /api/devices/{instance}/alarms/acknowledge`
`{ "type": 0, "instance": 3 }` acknowledges what the device reports for the object now.

`GET .../objects/{type}/{instance}/schedule` answers `{ "week": [[{ "time", "value" }], …], "exceptions":
[{ "when", "priority", "changes" }] }`, `GET .../calendar` the dates in words.

`GET /api/events` answers the alarm messages devices sent to this computer since the start, the last
500: `[{ "time", "device", "type", "instance", "object", "from", "to", "text" }]`. A device sends them
only to computers listed in its notification classes.

`GET .../alarms` also falls back on GetAlarmSummary where a device knows nothing newer. `can_acknowledge`
says whether the device told what an acknowledgement has to refer to.

`POST .../objects/{type}/{instance}/schedule` `{ "day": 0, "changes": "06:00 21.5; 18:00 17" }` writes
one day of the week (0 is Monday), logged; the values take the kind of the schedule's default value.

`POST /api/who-has` `{ "name": "…" }` asks the whole network for 3 s and answers `[{ "device", "type",
"instance", "name" }]`.

`GET /api/sites` lists the sites kept (`{ name, title, saved_ms, devices, read, open }`), one per
subnet, BBMD or adapter searched; `name` is that network, `title` the name the technician gave it or
`null`, `open` whether it is the site shown. `POST /api/sites/open` `{ "name": "…" }` shows one as
remembered. `POST /api/sites/rename` `{ "name": "…", "title": "…" }` names one (at most 80
characters; empty goes back to the network); a new search of the same network keeps the name.
`POST /api/sites/delete` `{ "name": "…" }` forgets one. `GET /api/site.json` downloads the remembered
site as a file, `GET /api/site.json?name=…` a kept one, `POST /api/site.json` takes such a file
back, keeps it and shows it; anything else gets `400`.

`GET /api/site/points` answers the points of every device read, by device ID:
`{ "21": { "points": [ … ] } }`, read since the start or else as remembered, without asking a device.

`GET /api/site.ede.csv` and `GET /api/site.ede-states.csv` download the same for all devices read
since bacnetio started, sorted by device. `409` if none was read. The interface reads the devices not read yet before it asks for it.

`GET /api/devices/{instance}/points-priorities.csv` downloads the point list CSV with two more
columns: `Winning priority` and `Set priorities` (`8=21.5 °C | 16=0`). It reads the priority array of
each commandable object from the device, one object after the other. `409` if the points were not
read yet.

`GET /api/devices/{instance}/alarms` reads the device's active alarms and faults
(GetEventInformation): `[{ "type": 0, "instance": 3, "object": "Analog input 3", "name": "…",
"state": "high limit", "unacknowledged": true }]`, an empty list when all is normal, `502` with the
reason when the device refuses.

`GET /api/devices/{instance}/points.csv` downloads the last reading as CSV for Excel (semicolon,
UTF-8 with BOM). `409` if the points were not read yet.

## Reference

`POST /api/devices/{instance}/reference` saves the device's last point reading as its reference
(one per device, in `references/` in the data folder), e.g. before a commissioning.

`GET /api/devices/{instance}/reference` compares the last reading with it:

```json
{
  "time_ms": 1793534400000,
  "changes": [
    { "object_type": 4, "instance": 1, "name": "Supply fan enable", "kind": "value", "was": "inactive", "drift": false },
    { "object_type": 0, "instance": 1, "name": "Supply air temp", "kind": "value", "was": "21.4 °C", "drift": true },
    { "object_type": 2, "instance": 9, "name": "New setpoint", "kind": "added", "drift": false }
  ]
}
```

`kind` is `value`, `status` (both with `was`), `added` or `removed`. `drift` marks analog values,
which change by themselves; a UI shows them but does not count them as changes. `time_ms` is
`null` when the device has no reference. Both need the points read first (`409` otherwise).

## Device clock

`POST /api/devices/{instance}/clock` `{ "date": "2026-11-01", "time": "09:30:05", "weekday": 7 }`
sets the device's clock to the viewer's local time (TimeSynchronization; weekday 1 is Monday).
The service is unconfirmed: `204` means it was sent. Read the device's `local-time` afterwards to
see whether it took it. Like every write it goes into the log (`action` `clock`).

## One object

`GET /api/devices/{instance}/objects/{type}/{instance}`

```json
{
  "type": 1, "instance": 1, "object": "Analog output 1",
  "name": "Heating valve", "description": "", "value": "42.5 %", "units": "%",
  "status": "", "error": null,
  "commandable": true,
  "priority_array": [
    { "priority": 1, "value": null, "mine": false },
    { "priority": 8, "value": "42.5 %", "mine": true }
  ],
  "relinquish_default": "10 %"
}
```

- `commandable`: the object has a priority array; writes need a priority
- `priority_array`: always 16 slots, priority 1 first; `value: null` is free; `mine` means bacnetio
  wrote it and has not released it yet. `null` as a whole when not commandable.
- `relinquish_default`: the value when all slots are free

`GET /api/devices/{instance}/objects/{type}/{instance}/value`: the present value, cheap enough to
poll every 2 s: `{ "value": "42.5", "status": "overridden", "source": "cov" }`. The first call
subscribes to changes (SubscribeCOV, unconfirmed, 5 min, renewed after 4); while the device
reports, the value comes from its reports without any request to it (`source: "cov"`). A device that
refuses the subscription is read each time (`source: "read"`, `status` then `null`). `value` has
no unit.

`GET /api/devices/{instance}/objects/{type}/{instance}/properties`: every property of the object,
for the technical details: `[{ "property": "present-value", "value": "21.4 °C", "error": null }]`.
Read with ReadPropertyMultiple for ALL properties; a device that cannot do that gets the common
properties one by one. Enumerations come by name (`no fault detected`, `alarm` for a notify type)
wherever the property alone names them, lists are cut after 20 entries. Event time stamps come as
`2026-11-02 14:03:00`, `never` when unset. Two fields only where they apply:

- `items`: a list or array entry by entry, in words: `["never", "never", "never"]` for event
  time stamps, `["too hot", "", ""]` for message texts.
- `bits`: a bit string bit by bit, first first: `[false, false, true, false]` for status flags
  (in alarm, fault, overridden, out of service).

`GET /api/devices/{instance}/objects/20/{instance}/trend`: the records of a Trend Log object, read
page by page with ReadRange: `{ "records": [["2026-11-01T09:30:00", 21.4]], "gaps": false,
"truncated": false }`. Times are the device's local time, without zone; `null` values are logged
failures. At most 20 000 records (`truncated` then true); `gaps` means the device dropped records.
Devices behind a router are not supported yet. `.../trend.csv` downloads the same as CSV.

## Write and release

Writing changes a running plant. The concept (section 6) asks the UI to unlock writing per
session and to show the priority array before and after; both endpoints answer with the object as
read again after the write.

`POST /api/devices/{instance}/objects/{type}/{instance}/write`

```json
{ "value": "21.5", "priority": 8 }
```

- `value` as typed: a number for analog objects (`,` or `.`), `active`/`inactive` (or `on`/`off`,
  `1`/`0`) for binary objects, a state number from 1 for multi-state objects. Other object types
  are refused for now.
- `priority`: 1 to 16. Leave it out only for objects that are not commandable. 8 (manual operator)
  is the usual choice for a technician.

`POST /api/devices/{instance}/objects/{type}/{instance}/release`

```json
{ "priority": 8, "only_mine": true }
```

Frees the slot (writes NULL), so the next lower priority or the relinquish default takes over.
With `only_mine` (as the list of held values sends it) the slot is read first and released only
while it still holds what bacnetio wrote; otherwise `409`, e.g. `priority 8 holds 60 % now, not
what bacnetio wrote: another system wrote over it, so it was not released`, and the value drops
out of the held values. Without it the slot is released as it is.

A refused write gives `502` with the device's reason, e.g. `the device refused: write access
denied`. Every attempt, refused or not, lands in the log.

## Held values and log

`GET /api/held`: values bacnetio wrote at a priority and has not released yet. Show these before
the user leaves, with a way to release each.

`GET /api/log`: every write and release, oldest first, including earlier sessions. The log is
appended to `writes.jsonl` in the data folder (`--data-dir`, default `%LOCALAPPDATA%\bacnetio` on
Windows, `~/.local/state/bacnetio` on Linux) and read again at start, so values still held survive
a crash or restart. Each entry keeps the device's address, so a write or release of a device the
connection does not know yet (after a restart, before any scan) opens a connection on the default
adapter and finds the device there, or by Who-Is if it sits behind a router.

Both return the same entries:

```json
[
  {
    "time": 1793534400, "device": 3001, "type": 1, "instance": 1, "object": "Analog output 1",
    "priority": 8, "action": "write", "value": "42.5", "error": null
  }
]
```

`time` is seconds since 1970 (UTC). `value` is `null` for a release. `action` is one of five:

- `write` — a value written at a priority, or directly on an object without a priority array.
- `release` — a priority given up again.
- `clock` — the device's clock was set. `value` is the date and time sent, the object is the device.
- `property` — any property written through `.../property`. `value` is the property and the value,
  such as `out-of-service = true`.
- `command` — a device command (warm or cold start, stopping or resuming communication), an
  acknowledged alarm or a changed day of a schedule. `value` says which, and the object is what the
  command acted on: the device, the object in alarm, or the schedule.

`GET /api/log.csv?offset_min=60` downloads the same entries as CSV for Excel (semicolon, UTF-8 with
BOM): time at the viewer's offset from UTC, device, object type, instance, priority, action, value
and why the device refused, if it did.

A page that shows the log has to take all five; the list grows with what bacnetio can change.

## Recording

Chosen points are read at a fixed interval and kept in memory (at most 32 points, 100 000 samples
per point, oldest dropped), with a backup file in the data folder (below).

`GET /api/recording`

```json
{
  "running": true, "interval_s": 10,
  "points": [
    {
      "device": 1001, "type": 1, "instance": 1, "object": "Analog output 1",
      "name": "Heating valve", "units": "%",
      "samples": [[1793534400000, 35.0], [1793534410000, null]]
    }
  ]
}
```

`samples` are `[milliseconds since 1970, value]`, oldest first. `null` means the device did not
answer or the value is not a number. Binary values are 0 and 1, multi-state values their state
number. All write requests below answer with this same object.

- `POST /api/recording/points` `{ "device": 1001, "type": 1, "instance": 1 }`: add a point; name
  and unit are read from the device once. `409` when 32 points are recorded already.
- `POST /api/recording/points/remove` with the same body: remove a point and its samples.
- `POST /api/recording/start` `{ "interval_s": 10 }`: start sampling, or change the interval of a
  running recording. At least 1 s; `409` without points.
- `POST /api/recording/mode` `{ "device": 1001, "type": 0, "instance": 1, "mode": "cov" }`: record
  the point by the device's change reports (`cov`) instead of polling it at the interval (`poll`,
  the default). `400` when the point is not in the recording.
- `POST /api/recording/stop`: stop sampling; the samples stay.
- `POST /api/recording/clear`: drop all samples, keep the points.
- While sampling, the recording is also saved every minute to `recording-utc.csv` in the data
  folder (times in UTC), so a crash or an empty battery loses at most a minute.
- `GET /api/recording.csv?offset_min=60`: CSV for Excel, one row per sampling round, one column
  per point. `offset_min` is the viewer's offset from UTC (`-new Date().getTimezoneOffset()`), so
  times are local.

## Plugins

Plugins (docs/concept-plugins.md, docs/plugins.md) are kept in the data folder under `plugins/`.
Without a data folder every request answers `409` `plugins need a data folder`.

- `GET /api/plugins`: `[{ "id": "check", "name": "Site check", "version": "1.0.0", "api": 1, "description": "…", "on": true }]`,
  by name.
- `POST /api/plugins` with the plugin file as the body (`text/html`, up to 8 MB): install and
  switch on; answers the plugin as above. `400` when the file is not a plugin bacnetio can run
  (description missing or wrong, other API version), `409` `plugin Site check 1.0.0 is installed
  already` when the id is taken. `POST /api/plugins?replace=true` replaces it.
- `POST /api/plugins/{id}/switch` `{ "on": false }`: switch off or on.
- `POST /api/plugins/{id}/remove`: remove the plugin and its store.
- `GET /api/plugins/{id}/page`: the plugin's page for its frame, with the policy
  `sandbox allow-scripts; default-src 'none'; …`: it runs its own script and reaches nothing.
  `409` while switched off.
- `GET /api/plugins/{id}/store` and `POST` with JSON (up to 1 MB): the plugin's own store,
  `null` until it saved something.

`404` `no plugin … is installed` for an unknown id.
