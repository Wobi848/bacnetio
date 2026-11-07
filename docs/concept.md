# Concept: bacnetio (BACnet browser)

Updated: 2026-11-01 · Status: **draft, everything open** (answers and open questions in section 13,
current proposals in section 14)

Nothing here is fixed. Every choice below is a proposal with its reason, to be challenged and
replaced if something better turns up. The goal is the best tool, not the most familiar stack.

## 1. Why

There are hardly any usable free BACnet tools. YABE is the best known, but often fails: it finds
nothing, finds only some devices, or aborts while reading. The causes are almost always the same,
and YABE names none of them:

| Cause | What you see |
| --- | --- |
| wrong network adapter (Wi-Fi, VPN, Hyper-V/Docker active) | Who-Is goes to the wrong network, no answers |
| port 47808 taken by another program | starts fine, nothing arrives |
| firewall drops incoming UDP | Who-Is goes out, I-Am never arrives |
| devices in another subnet | broadcasts don't cross routers, devices missing |
| devices behind BACnet routers (MS/TP) | only IP controllers appear, field devices missing |
| large object lists, segmentation, broken ReadPropertyMultiple | aborts while reading |
| duplicate device instances | devices "jump", values belong to the wrong device |

bacnetio is built from the standard (ASHRAE 135, ISO 16484-5), not derived from existing browsers.
License MIT. YABE (GPL) is read for reference only, no code is taken. Commercial browsers are not
decompiled.

## 2. Goal

**A BACnet browser that anyone can use without explanation: it finds devices reliably, reads them
cleanly, and when something fails it says why in plain words.**

Users: service technicians and integrators with a laptop or phone on a building network. Typical
jobs: see what is on the network, check data points, document a point list. No continuous
operation (trending, forwarding) in version 1.

## 3. Design: a tool like mbusio

The first field test (1.11.2026, Kieback&Peter DDC4000 over a HOOC VPN, 170 points) showed that
the phone-style flow (one screen per step, big cards) does not hold a real plant: long path names,
points of one plant part torn apart by type, no overview, English on a German PC. bacnetio now
follows mbusio's interface, which the maintainer sets as the standard: `~/Coding/mbusio`,
`docs/concept-operation.md` §11–§17 and `.claude/agents/designer.md` there.

### 3.1 Principles (taken from mbusio)

1. **Self-explanatory instead of explained.** State, meaning and next action on the element itself.
2. **Every page says where you are:** a title, one sentence, and the one next step at the top right.
3. **List and details side by side** on the desktop; the phone stacks them.
4. **Working context stays:** selection, filter, scroll and expanded groups survive actions.
5. **Dense, but readable:** 14 px text, compact table rows, frames only around cards.
6. **Buttons in tiers:** one filled button per bar (the next step), `tonal` beside the main path,
   `quiet` for rare things. Every button looks like a button.
7. **Colours for problems:** orange = note, red = fault or danger, green = OK. Never colour alone.
8. **Keyboard:** `1`–`6` for the areas, `/` for the search across the site, arrows in lists, Enter opens, Escape closes.
9. **Language and theme follow the system** until chosen in Settings.
10. **Desktop first**, phone fully usable (tabs instead of the sidebar, 16 px margins).

### 3.2 Layout

```text
┌──────────────┬────────────────────────────────────────────────────────────────┐
│ bacnetio     │ Data points                                 [Next step: …]     │
│ [Ethernet ▾] │ DDC4000_#21 · Kieback&Peter DDC4000 · 192.168.10.21            │
│ ● Connected  ├────────────────────────────────────────────────────────────────┤
│   [Change]   │ [Search…] [All 170|Faults 1|Overridden 1]   [Read again] [CSV] │
│──────────────│ ▾ SY_Datsi.01                     │ procBar                    │
│ ▣ Overview 0 │     state      MSI   Running      │ 100 %                      │
│ ⌕ Devices  1 │     procBar    AI    100 %        │ Priority array, write,     │
│ ≡ Points   2 │ ▸ HZ_Kessel.01    12 · 1 fault    │ properties …               │
│ ◷ Recording 3│                                   │                            │
│ ✎ Log      4 │                                   │                            │
│ ⚙ Settings 5 │                                   │                            │
├──────────────┴────────────────────────────────────────────────────────────────┤
│ ● Ready · Ethernet 192.168.10.5/24 · port 47808                               │
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Sidebar:** one card with the site as a select field (its name, or its network while it has
  none; the known sites and *All sites …*), below it the network once and the state of the search,
  beside them *Search settings* (adapter, plant subnet, BBMD, port, ID range); then the search
  across the site, the areas, Settings at the bottom. Dark in both themes.
- **Overview:** what the network and the open device look like, with the next step.
- **Devices:** the search runs in place with progress; table of devices, details of the chosen one
  beside it (vendor facts, clock, reference), *Open data points* as the next step.
- **Data points:** grouped by the plant part in the object names. Many controllers name objects as
  paths (`/21/000/00/00/SY_Datsi.01/state`); the part common to all is left out, the next segment
  becomes the group, the rest the name. Without paths the groups are the object types. Groups with a
  fault or an override show it while collapsed. Multi-state values show their state texts.
- **Recording, Log:** as before, as pages of their own.
- **Status bar:** what bacnetio is doing right now.

### 3.3 Functions and where they live

The reference for "never lose a function" (AGENTS.md): a change that moves or replaces one updates
its row. UI tests are in `tests/ui/`, Rust tests in `tests/` and next to the code, UI logic in
`tests/web/`.

| Function | Place | Test |
| --- | --- | --- |
| Choose the network adapter | sidebar, *Search settings*: the adapter by name and address; an empty device list offers it too | `ui/14-details`, `ui/33-adapters`, `interfaces.rs` |
| Search: Who-Is and every address asked directly, at the same time | workspace, next step; while values are held a tool beside it, on a phone in the toolbar | `ui/01-search`, `ui/03-write`, `scan.rs` |
| Devices appear while the search runs, names read at once | table of the site | `scan.rs` (`devices_show_while_the_scan_runs`) |
| Object lists read during the search, count and progress per device | table of the site, *Objects*; tree | `ui/01-search`, `ui/16-requests`, `scan.rs` (`reads_every_object_list_during_the_search_when_asked`) |
| Plant subnet (VPN), BBMD, port, device ID range (BACnet/IP) | sidebar, *Search settings* | `ui/04-reload`, `bbmd.rs`, `scan.rs` (range) |
| Choose BACnet/IP or MS/TP (RS-485) | sidebar, *Search settings*, first row (not on Android) | `ui/37-mstp` |
| Serial ports, USB adapters first with their name | *Search settings* with MS/TP | `mstp.rs` (`usb_adapters_come_first`), `ui/37-mstp`, `api.rs` |
| Listen to an MS/TP trunk without sending: speed, stations, a free address | *Search settings* with MS/TP, *Listen to the trunk* | `tests/mstp.rs`, `mstp.rs`, `ui/37-mstp` |
| Join an MS/TP trunk and search it; refused before listening, at another speed or an address in use | *Search settings* with MS/TP, *Join the trunk and search*; *Search again* | `tests/mstp.rs` (`joining_a_trunk_finds_its_device_and_reads_it`), `mstp.rs` (`joining_needs_…`), `ui/38-mstp-join` |
| Leave the trunk; asks first while values are held; joining and leaving logged | sidebar head while on a trunk; *Written* log | `ui/38-mstp-join` |
| A value held on a trunk, after a restart: its address shown, releasing asks to join the trunk first | *Held by you* | `session.rs` (`a_device_is_reached_only_over_the_link_it_was_written_on`), `ui/39-mstp-held` |
| Read, write, release, COV over MS/TP | as on IP | `tests/mstp.rs` (`writing_releasing_and_cov_reports_work_over_the_trunk`), `ui/38-mstp-join` |
| BBMD tables read (BDT and FDT), never written; a device that is none says so | device details, *BBMD tables* | `ui/24-bbmd`, `tests/bbmd.rs` (both tables, no BBMD), `web/ui.test.js` |
| Device by IP address | table of the site, toolbar, *Device by IP …* (BACnet/IP only, not on a trunk) | `ui/10-locate` |
| Routers and the networks behind them | tree and strips above the table | `scan.rs` (routers) |
| Diagnosis in plain words (firewall, port taken, link-local, duplicates …) | strips above the table | `diagnosis.rs`, `scan.rs` (taken port) |
| Device list as CSV | table of the site, toolbar, *CSV* | `ui/13-exports`, `export.rs` |
| Device facts (vendor, firmware, protocol, clock …) | details of the chosen device | `ui/09-device` |
| Set the device clock to this computer's time, logged | device details, behind the unlock | `ui/09-device`, `storage.rs` |
| Read all points, with fallbacks (object list in parts, no RPM), sized to the device's limits, in the background with progress | table of a device | `read.rs`, `fallback.rs`, `limits.rs`, `ui/02-points` |
| Request log of a reading | device details, *Show request log* | `ui/16-requests`, `read.rs` |
| Group points by module (path names) or kind; plant parts and types in the tree | table of a device, tree | `ui/02-points`, `web/ui.test.js` |
| Plant view from the device's own Structured View objects (type 29, Subordinate_List), nested, with what no view holds; names group where a device has none | tree, table of a device | `ui/23-plant`, `web/ui.test.js`, `tests/structure.rs`, `structure.rs` (decoding) |
| Search points; filter faults, overrides, changes, object type | table of a device, toolbar | `ui/02-points`, `ui/18-sort-filter`, `web/ui.test.js` |
| Object list with description and ID in columns of their own, search marked | table of a device | `ui/25-object-list` |
| Live values in the list, only for the rows on screen | table of a device, *Live values* | `ui/25-object-list` |
| Tree by type or by plant, remembered | above the tree | `ui/12-workspace`, `ui/23-plant` |
| Open or close all groups | table of a device, *More …* | `ui/25-object-list` |
| Sort devices and points by any column | column heads | `ui/18-sort-filter`, `web/ui.test.js` |
| Multi-state values by their state texts | table, details, write field | `ui/02-points`, `read.rs` |
| Read the device again | table of a device, toolbar, *Read again* | `ui/14-details` |
| Point list as CSV | table of a device, toolbar, *CSV* | `ui/13-exports`, `export.rs` |
| Point list as EDE file | table of a device, toolbar, *EDE* | `ui/13-exports`, `ede.rs` |
| EDE state text file, per device and for all | table of a device, toolbar, table of the site, toolbar | `ui/13-exports`, `ede.rs` |
| EDE file of all devices, reading those not read yet | table of the site, toolbar, *EDE (all devices)* | `ui/13-exports` |
| Excel files beside each CSV: devices, a device's points (also with priorities), the whole site with a sheet per device, log, recording; heads in the page's language | export menus beside *CSV* | `workbook.rs`, `tests/api.rs` (`excel_files_check_what_the_page_sends`), `tests/web/excel.test.js`, `ui/45-excel` |
| Print view of a device's points, with the list's filter and view, saved as PDF by the browser | table of a device, *More → Print …*; Ctrl+P in the list | `ui/40-print` |
| Own grouping pattern per site, with a preview; kept with the site and over a new search | table of a device, *More → Grouping …* | `ui/43-grouping`, `tests/web/grouping.test.js`, `site.rs` (`a_grouping_pattern_stays_with_its_site`) |
| Compare two devices by point name: differences, only left, only right; read both again | table of a device, *More → Compare with …* | `ui/41-compare`, `tests/web/compare.test.js` |
| Capture bacnetio's own traffic and save it as pcapng for Wireshark, MS/TP with every frame on the line | *Settings*, card *Capture for Wireshark*; status bar while it runs | `tests/capture.rs`, `tests/mstp.rs` (`a_capture_on_a_trunk_holds_whole_frames`), `pcap.rs`, `ui/42-capture` |
| Choose points and change one property of all at once: present value with priority, release, out of service, COV increment, description, alarm limits; behind the unlock, one more question, each write logged and held | table of a device, *Select*, *Change selected …* | `tests/web/change-many.test.js`, `ui/44-change-many` |
| Point list as CSV with the winning priority and set slots | table of a device, toolbar, *CSV with priorities* | `ui/13-exports`, `export.rs` |
| Active alarms of a device (GetEventInformation) | device details | `ui/09-device`, `alarms.rs` (test) |
| Save a reference, show what changed since | table of a device, toolbar | `ui/06-reference`, `reference.rs` |
| Value live: COV where offered or asked every 2 s, chosen per point | point details | `ui/14-details`, `watch.rs` |
| All 16 priorities: who holds what, what wins | point details | `ui/03-write`, `ui/19-priorities` |
| Write and release with priority, after the session unlock | point details | `ui/03-write`, `write.rs` |
| Every property of an object | point details, *Show technical details* | `ui/14-details`, `read.rs` |
| Trend logs of the device as chart and CSV | point details of a trend log | `ui/08-trend`, `trend.rs` |
| Recording compared: one time axis, a chart per unit, window and zoom, lowest, highest, mean, on-share, changes, gaps | *Recording* | `ui/05-recording`, `web/ui.test.js` |
| Recording: add, interval, start, stop, remove, clear, CSV, backup | point details, *Recording* | `ui/05-recording`, `ui/13-exports`, `record.rs` |
| Values still held, release one or all, also after a restart | *Held by you*, status bar | `ui/03-write`, `storage.rs` |
| Log of every write, release and clock setting; refused ones with the reason; as CSV | *Held by you*, *Export log (CSV)* | `ui/03-write`, `ui/09-device`, `ui/21-tools`, `export.rs` |
| Warning before closing while values are held; *Leave the site* offers to release them | browser, *Held by you* | `ui/15-leaving`, `ui/20-site` |
| The site at a glance: tree with marks, next step | workspace | `ui/12-workspace` |
| Language and theme, following the system | settings | `ui/07-settings` |
| Keys `1`–`6`, `/`, arrows, Enter, Escape | everywhere | `ui/11-keys` |
| A reload keeps area, devices, open device, settings | browser | `ui/04-reload` |
| Last search and points read stay after a restart, marked as remembered | devices, data points | `ui/17-site-memory`, `site.rs` (test) |
| Only pages of this bacnetio may use the API | server | `api.rs` (`foreign_pages_are_refused`) |
| Layout: nothing clipped in any language, width or theme | every page | `ui/27-layout` |
| Command line: `scan`, `read`, `write`, `release`, `interfaces` | terminal | `scan.rs`, `read.rs`, `write.rs` |
| Warning before a write or release that would not act | point details | `ui/19-priorities` |
| Problems across the devices read: faults, alarms, overrides, out of service, offline; read all devices | *Site problems* | `ui/20-site` |
| Search across the site | sidebar | `ui/20-site` |
| Stop a running search or reading | status bar | `ui/26-stop` |
| Watch list: points across devices with live values | *Watch list* | `ui/20-site` |
| Change a device's ID, after the unlock, logged, with the search following | device details | `ui/21-tools`, the field checklist for a device that allows it |
| Warm and cold start, stop and resume communication, with password, logged | device details, *Device commands* | `ui/22-commands`, the field checklist |
| Acknowledge an alarm | device details, *Active alarms* | the field checklist (the simulators report no alarms) |
| Schedule and calendar in words; a day of the week changed after the unlock, logged | point details of such an object | `ui/22-commands`, `tests/schedule.rs`, `schedule.rs`, the field checklist |
| Ask the network who has an object of a name (Who-Has) | search, *Ask the network …* | `ui/22-commands` |
| Alarm messages from devices (event notifications), and GetAlarmSummary for older devices | *Site problems*, device details | `alarms.rs` (test), the field checklist |
| Device behind a router by network and MAC | *Device by IP …* | `ui/21-tools` (input), `locate.rs`; the field checklist for the real thing |
| Properties with data type, updated every 10 s, changes marked | point details, *Show technical details* | `ui/21-tools` |
| Change any property with a plain value, logged; the device decides | technical details, after the unlock | `ui/21-tools`, `ui/22-commands`, `storage.rs` |
| The priority that wins on each point | table of a device, *Read priorities* | `ui/21-tools` |
| Values reported unasked (unconfirmed COV) count as live | point details | `watch.rs` |
| Several sites kept, opened again, named, removed, saved to and loaded from a file | the site field in the sidebar (known sites, *All sites …*); a site without a name: *All sites …* opens its name field | `ui/21-tools`, `site.rs` |
| Android app with the same UI | phone | not automated (concept section 9) |

## 4. Scope

### Version 1

- BACnet/IP (UDP, Annex J) over IPv4
- Find devices: Who-Is/I-Am, also with range, local, directed to an IP, and via BBMD (foreign
  device)
- Devices behind BACnet routers: discover networks (Who-Is-Router-To-Network), address devices by
  network number and MAC
- Read: object list, object properties (name, description, present value, units, status flags,
  …), ReadProperty and ReadPropertyMultiple, with segmentation
- Plain-word diagnosis (section 7)
- Write: WriteProperty with priority, relinquish, show the priority array (section 6)
- UI in English, German, French, Italian and Spanish, the system's language by default; French,
  Italian and Spanish enter the menu after a review ([concept-languages.md](concept-languages.md));
  point list export as CSV
- Windows and Linux, **without administrator rights** (section 7.4)
- BACnet MS/TP directly over a USB RS-485 adapter, as its own connection type, not on Android
  ([concept-mstp.md](concept-mstp.md))

### Later (in this order, section 12)

Built since: recording (polled or COV, chart, CSV), the Android app, COV live values, sites kept
and compared with a reference, trend logs, alarms and events, and Excel files beside the CSV
([concept-excel.md](concept-excel.md)). Nothing of this list is open.

### Later or never

- Raspberry Pi: not a target
- BACnet/IPv6, BACnet/SC, BACnet Ethernet: not planned
- Acting as a BACnet server: only the minimum (a device object for devices that ask back)

## 5. Stack (open)

### 5.1 Requirements that drive the choice

- One codebase for Windows, Linux and Android
- Single file, no installation on the desktop
- Usable from a phone on the same network (laptop on the plant network, phone in hand)
- Raw UDP sockets with control over binding and broadcast
- Easy to test the protocol without a network

### 5.2 Core language

| Option | For | Against |
| --- | --- | --- |
| **Rust** (chosen 2026-11-01) | single static binary, cross-compiles to Windows/Android, fine-grained socket control, strong typing for a binary protocol, same as mbusio | slower to write than Go/C# |
| Go | very easy cross-compile, simple concurrency | Android only via gomobile, weaker typing for protocol enums |
| C# / .NET | YABE lives here, good tooling | larger runtime, Android via MAUI is heavy |

### 5.3 UI technology

| Option | For | Against |
| --- | --- | --- |
| **Local web server + web UI** (chosen 2026-11-01) | same UI on desktop, from any phone, and in an Android WebView | needs a browser tab on desktop |
| Tauri 2 | native window on desktop, also Android | Linux needs WebKitGTK, no access from a phone |
| egui (native Rust UI) | one binary, no web stack | looks technical, poor on phones, no remote access |
| Flutter | great mobile UI | second language (Dart) plus FFI to the core |

The web UI can still be wrapped in a native window later if a desktop window is wanted; that does
not change the core.

### 5.4 Frontend

Open, to be decided with the UI concept before M2. mbusio uses plain JavaScript modules without a
build step (about 11,600 lines); that keeps tooling minimal but makes a consistent simple UI harder
as it grows. Candidates: plain JS again, Preact + htm (no build step, components), Svelte (build
step, very little code). Criterion: which one makes a simple, consistent UI easiest to keep simple.

### 5.5 Protocol library

**rusty-bacnet** (github.com/jscott3201/rusty-bacnet, MIT, version 0.15, active) is a large crate
family: `bacnet-encoding`, `bacnet-services`, `bacnet-types`, `bacnet-bip-core` (sans-IO BACnet/IP,
BBMD), `bacnet-client` (TSM, segmentation, discovery), even an MS/TP core.

| | own encoding | rusty-bacnet |
| --- | --- | --- |
| time to discovery | weeks | days |
| control over fallbacks and error cases | full | through their API |
| risk | own bugs, but testable | pre-1.0, fast changes, essentially one maintainer |

Proposal: in M0, test rusty-bacnet for a day against captures and `bacserv` (section 11). If it
holds up, use its encoding crates with a pinned version and write transactions, fallbacks and
diagnosis ourselves, because that is what makes bacnetio better than YABE. Otherwise write the
encoding ourselves.

**M0 result (2026-11-01, local spike, rusty-bacnet 0.15.0):**

- Client and its own simulated server (`bacnet-server`) talk over real UDP on Linux: directed
  Who-Is/I-Am, ReadProperty, wildcard probe (Device 4194303) for the firewall-friendly sweep
  (section 7.4) all work.
- Segmentation works: an object list of 10,001 entries (50 KB) arrives whole in about 7 ms.
  Without server segmentation the device answers abort "segmentation not supported"; reading
  index 0 and element by element then works, so the fallback in section 6 is possible.
- The transport already handles the per-OS socket rules from section 7 (Linux broadcast
  listeners, Windows `SO_EXCLUSIVEADDRUSE`, port sharing by address), with design notes and tests.
- The client covers far more than the encoding: TSM, segmentation, routed devices (DNET/DADR),
  COV renewal, ReadRange, write priority. Using `bacnet-client` as a whole is realistic.
- **Android build fails** in `bacnet-transport` (`ipi6_ifindex` is `i32` on Android, `u32`
  expected). A one-line cast fixes it; everything then compiles for `aarch64-linux-android`.
  Needs an upstream fix or a `[patch]` until then.
- Loopback quirk: a server bound to `0.0.0.0` answers from `127.0.0.1`, so tests on one host use
  one IP with different ports.
- Windows: not built here (no MinGW). Windows builds run in GitHub Actions (MSVC), as in mbusio.

Still open for M0: a real site, and the user's Windows laptop without admin rights (section 7.4).

**M1 status (2026-11-01):** `bacnetio scan` (Who-Is, direct sweep, diagnosis) and `bacnetio read`
(object list with entry-by-entry fallback, ReadPropertyMultiple with shrinking batches and
single-read fallback, CSV export) and `--bbmd` (foreign device registration, refusal and silence
reported in words) work against simulated devices. Not yet: Who-Is-Router-To-Network (a list of
networks; routed devices already appear through the global Who-Is), field test. The ReadPropertyMultiple fallback has no automated test,
because the simulated server always supports it.

**M2 backend status (2026-11-01):** `bacnetio` without a command serves the JSON API from
[api.md](api.md) on 127.0.0.1:8081 and opens the browser: adapters, scan with progress (polling,
no server-sent events yet), points, CSV, one object with priority array, write, release, held
values and a write log. The UI goes into `web/` and is served by bacnetio itself; there is
deliberately no CORS, because any web page could otherwise write to the plant through the API.
The held values are derived from the write log, not kept separately. Writing is tested against a
simulated analog output (priority 8, release to relinquish default).

**Status 2026-11-01, later:** web UI built from the prototype (English and German), routers
listed via Who-Is-Router-To-Network during the Who-Is wait, recording of chosen points (polled,
kept in memory, one chart per point, CSV with local times). Recording moved into version 1
because the user asked for it. Notes from the core are enums with codes, so the UI translates them.

**Status 2026-11-01, evening:** Android app (foreground service + WebView, multicast lock,
bacnet-transport patched at build time only), write log on disk (`writes.jsonl`, held values
survive restarts), device list CSV, point search, live object value, German texts for notes and
known errors. Tests: unit, loopback integration (scan, read, write, BBMD, routers, recording,
ReadPropertyMultiple fallback via a rejecting proxy), API without network, UI logic under
`node --test`. Still untested: Windows, a real site, a phone, reading devices behind a real router.

**Status 2026-11-01, end of the day:** also trend logs read from devices (ReadRange via the
library's log reader), references per device to compare with later (JSON files in the data
folder instead of SQLite: one file per device is all that is needed so far), device details and
setting the device clock, writing objects without priority array, CLI `write`/`release`, scan by
device ID range, opening a device by address, recording backup, held values releasable after a
restart (the write log keeps each device's address). Not built then: Who-Has search, a packet
console, alarms (GetEventInformation), devices behind routers for trend logs. Built since: Who-Has
and active alarms with acknowledging; instead of a packet console, the capture for Wireshark
(concept-toolbox.md part 3).

Revised proposal: use `bacnet-client` and `bacnet-transport` (pinned), build discovery strategy,
fallbacks, throttling and diagnosis on top. Keep rusty-bacnet types out of the UI-facing API, so
a swap stays possible.

### 5.6 Layers

```text
 shells     Windows · Linux · Android app
 app        HTTP server, REST + server-sent events, embedded UI
 browser    discovery, reading, fallbacks, throttling, diagnosis, export
 net        UDP sockets, adapter selection, BBMD registration (per platform)
 protocol   BVLC / NPDU / APDU encoding, services, transactions, segmentation — no I/O
```

`protocol` knows no sockets, no async runtime and no clock: it gets bytes and "now is t" and
returns bytes and events (sans-IO), so it is fully testable with captures and simulated time. Only
`net` touches sockets. `browser` holds the tool's logic and knows nothing about HTTP. Dependencies
point down only. One crate with modules at first; split only when a part is needed elsewhere.

## 6. Protocol

| Part | Content | Note |
| --- | --- | --- |
| BVLC (Annex J) | Original-Unicast/-Broadcast, Forwarded-NPDU, Register-Foreign-Device, Distribute-Broadcast-To-Network, Result, Read-BDT/-FDT | In a Forwarded-NPDU the real sender is the address **inside** the packet, not the UDP sender (the BBMD). Replies go to the real sender. |
| NPDU (clause 6) | DNET/DADR, SNET/SADR, hop count, network messages | Who-Is-Router-To-Network, I-Am-Router-To-Network, Reject-Message-To-Network |
| APDU | confirmed/unconfirmed request, simple/complex ACK, segment ACK, error, reject, abort | error/reject/abort reasons in words |
| Services v1 | Who-Is, I-Am, Who-Has, I-Have, ReadProperty, ReadPropertyMultiple | |
| Services later | WriteProperty, SubscribeCOV, COV notification, ReadRange, GetEventInformation | |
| Segmentation | receive segmented replies with window; sending not needed yet | respect max APDU and segmentation from I-Am |
| Transactions | invoke IDs per device, timeout, retries | longer timeouts for MS/TP behind routers |

**Principles for reading:**

- **Gentle:** few parallel requests per device and per routed network. Small controllers and MS/TP
  devices choke on floods. Slower but complete beats fast and broken.
- **Fall back, don't abort:** object list whole first; on error or "segmentation not supported",
  read index 0 (length), then element by element. ReadPropertyMultiple first; on reject/error, single
  ReadProperty. What a device can't do is remembered for that device.
- **Writing is deliberate.** A wrong value in a running plant does real damage. Writing must be
  unlocked per session, shows the priority array before and after, and every write is logged.
  Before leaving, bacnetio lists every value it wrote and has not relinquished yet.

## 7. Network

### 7.1 Ports and sockets

Many devices send I-Am as a broadcast to port 47808, even if the Who-Is came from another port.
Not listening on 47808 means missing them. But 47808 may be taken by another BACnet program.

Proposal (verify in M0):

- Socket A on 47808 with address reuse (SO_REUSEADDR, plus SO_REUSEPORT on Linux) to receive
  broadcasts. If that fails: say "port 47808 is used by …" where the program can be found.
- If the port is exclusive, use socket A for requests too. If shared, use a separate port (e.g.
  47809) for requests, because confirmed replies go to the sender port.
- Port configurable under *Advanced* (some sites use 47809+ for separate BACnet networks).

### 7.2 Adapter selection

Picked automatically, changeable in one click (section 3.2). Broadcasts go to the directed
broadcast address of the chosen network (e.g. 192.168.10.255), not 255.255.255.255, which leaves
through only one adapter when there are several.

| Platform | Behaviour |
| --- | --- |
| Windows | binding to the adapter IP still receives broadcasts. Firewall prompts on first start; diagnosis checks whether incoming UDP arrives and otherwise shows how to allow it |
| Linux | binding to a specific IP receives **no** broadcasts. Bind to 0.0.0.0 and assign packets to the chosen network by source address |
| Android | section 9 |

### 7.3 Other subnets

- **Foreign device:** register with the BBMD with a TTL, renew in time; Who-Is goes as
  Distribute-Broadcast-To-Network to the BBMD; a NAK is shown in words.
- **Directed:** Who-Is as unicast to an IP or IP range, for networks without a BBMD.
- **Read BDT/FDT:** shows which BBMDs exist and who is registered.

### 7.4 Without administrator rights

YABE finds no devices on the user's Windows laptop unless run as administrator. Most likely cause:
without admin rights nobody can allow the firewall prompt, so Windows drops incoming UDP that does
not answer something we sent. bacnetio must work without admin rights and without a firewall rule.

The Windows firewall lets through UDP replies from an address and port we sent to, and (by
default) unicast replies to our own broadcasts for a few seconds. Broadcast I-Am answers are
dropped. Proposal (verify on the user's laptop in M0):

1. Try the normal way: Who-Is broadcast, listen.
2. If no I-Am arrives, or in addition: **unicast sweep** of the chosen subnet with
   ReadProperty(Device 4194303, Object_Identifier). 4194303 is the wildcard every device must
   accept. The reply comes from the IP and port we asked, so the firewall lets it through.
3. Behind routers: requests go to the router's IP, replies come from it, also let through.
4. With a BBMD: register as foreign device; everything arrives from the BBMD we talk to.
5. The diagnosis says what happened: "Broadcast answers are blocked by the firewall; found 12
   devices by asking each address directly."

## 8. Diagnosis

Shown where the problem happens (principle 5), not in a separate area.

| Observation | Message (roughly) |
| --- | --- |
| bind on 47808 fails | "Port 47808 is used by X. Close the other BACnet program or …" |
| own Who-Is broadcast not seen as echo | "Broadcast does not arrive: wrong adapter or firewall." |
| I-Am from outside the chosen network | "Answers from 10.1.2.0/24: that network is on another adapter, or the mask is wrong." |
| no answer, but other BACnet traffic seen | "There is BACnet here, but nobody answers Who-Is: check range, port or BBMD." |
| no BACnet traffic at all | "No BACnet traffic on this network. Check cable, VLAN, port." |
| same device instance from two addresses | "Device ID 1001 appears twice: 192.168.1.20 and network 5 / MAC 12." |
| same network number from two routers | "Network 5 is reported by two routers: configuration error in the site." |
| router reports network unreachable | "Network 7 behind router 192.168.1.5 unreachable (MS/TP trunk?)." |
| error / reject / abort | class and code in words with a hint, not as a number |

A packet console (live, decoded, click a packet to see every field) is not built; the request log
per device (concept-explorer section 2, point 5) says what was asked and what came back, and
bacnetio's own traffic is captured for Wireshark under *Settings* (concept-toolbox.md part 3).

## 9. Android

Same pattern as **mbusio-android**: a Kotlin app ships bacnetio (Rust, `aarch64-linux-android`)
as `libbacnetio.so`, runs it as a separate process in a foreground service, and shows the web UI
in a WebView on `127.0.0.1`. A crash of the core does not take the app down.

The Kotlin side additionally has to:

1. **Hold a MulticastLock** while searching. Without it Wi-Fi drops broadcasts and no I-Am arrives.
   The lock is device-wide, so it covers the Rust process too.
2. **Pick the network (Ethernet via USB-C):** Android routes traffic over Wi-Fi or mobile data by
   default, not over an Ethernet without internet. `Network.bindSocket` only works in-process.
   *Open (M4):* is binding to the Ethernet adapter's IP in the Rust process enough? If not, Kotlin
   opens the UDP socket via `Network.bindSocket` and passes the file descriptor over a local socket
   (SCM_RIGHTS), like the USB bridge in mbusio-android.
3. **Static IP:** building networks often have no DHCP, and many phones offer no static IP for
   Ethernet. *Open (M4):* check on the actual phone; otherwise Wi-Fi via an access point in the
   building network.

Port 47808 is above 1024, no root needed.

## 10. Data and reuse

Version 1 stores no site: found devices and read values live in memory and can be exported.
Settings (adapter, port, BBMD, language) live in a file in the data directory. Saving and comparing
sites comes later (SQLite, as in mbusio).

From mbusio we take patterns, not the UI: server with embedded UI, command line flags
(`--lan`, `--token`, `--no-browser`, `--data-dir`), CSV export, light/dark theme, the Android shell.
Copy at first; a shared crate only if both projects keep needing the same change, because it
couples two release cycles. Default web port not 8080, so bacnetio and mbusio can run side by side.

## 11. Testing

1. **Captures as test cases:** Wireshark captures from real sites, split into packets. Each case:
   bytes → expected decoding, and back. Every field problem (abort, odd device) becomes a capture
   and a test case.
2. **Logic without a network:** since `protocol` and `browser` have no I/O, discovery and fallbacks
   run against a simulated device with simulated time (timeouts without waiting).
3. **Real counterpart:** `bacserv` from bacnet-stack as a test device (separate program, no license
   issue), plus a routed setup with two instances in Docker networks for routing and BBMD.
4. **Field test** before each milestone in at least one real site, side by side with YABE.
5. **Usability test** before M3: a first-time user, no help, acceptance test from section 3.1.

## 12. Milestones

| | Goal | Done when |
| --- | --- | --- |
| M0 | decide protocol library (5.5) and sockets (7.1) | rusty-bacnet evaluated; Who-Is/I-Am as CLI on Linux and Windows, also with port taken |
| M1 | CLI: find and read | `bacnetio scan` lists devices (local, directed, BBMD, behind routers); `bacnetio read <device>` reads object list and values with fallbacks and segmentation; diagnosis as text |
| M2 | UI concept, then web UI | clickable mockups agreed; flow from section 3.2; CSV export; write with unlock, priority, relinquish, log; Windows/Linux builds |
| M3 | field-ready | field and usability tests passed; first public release |
| M4 | Android | app as in section 9, open points solved |
| M5 | record, COV | record chosen points (poll/COV), chart, CSV; live values |
| M6 | save sites, trends | SQLite, compare two snapshots, Excel export, ReadRange |

## 13. Answers and open questions

Answered 2026-11-01:

- YABE finds no devices without administrator rights → section 7.4.
- Tests on real sites and on a certified BACnet DDC controller are possible.
- All kinds of vendors in the field; MS/TP trunks behind routers are common.
- No Raspberry Pi.
- Writing in version 1.

Open:

1. A Wireshark capture of YABE finding nothing (and one of a working discovery) as first test cases.
2. Android: which phone, is there an Ethernet adapter? Is Wi-Fi in the building network realistic?
3. ~~Recording values: needed in version 1, or right after?~~ In version 1 (built).

## 14. Current proposals

All open; each can be overturned with a better reason.

| # | Proposal | Reason |
| --- | --- | --- |
| 1 | Build from the standard, don't decompile | open standard; licenses (GPL, commercial) |
| 2 | Simple-first UI, one thing per screen (section 3) | mbusio shows too much at once |
| 3 | Rust core, local web UI (**chosen**) | one codebase for desktop, phone and Android |
| 4 | Sans-IO protocol core | testable with captures and simulated time |
| 5 | Android like mbusio-android, no Tauri | Kotlin code needed anyway; proven setup |
| 6 | Routing (devices behind routers) in version 1 | otherwise most field devices are missing |
| 7 | Write in version 1, unlocked per session and logged | needed for commissioning; avoid accidental damage |
| 8 | English everywhere, German as UI translation | code, docs, commits, GitHub, CLI and UI default in English |
