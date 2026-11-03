# Concept: one workspace for a plant

Status: **approved and built**, 1.11.2026 (quality plan step 5 C). Decisions from section 9: "By type" stands beside the plant parts; only opened devices are read by themselves, *Read all devices* does the rest. Builds on
[concept-explorer.md](concept-explorer.md). Clickable mockup: [mockup/workspace.html](mockup/workspace.html)
(open it in a browser; static data, nothing talks to a device).

## 1. Why

Today bacnetio is a row of pages: overview, devices, data points, recording, log. Browsing a plant
means going back and forth between the device list and the point list, and nothing shows the plant
as a whole: which points are faulty or overridden across all devices, what you still hold, where a
name appears. The maintainer wants one workspace, as technicians know it from the K&P BACnet browser
and YABE, with mbusio's look.

## 2. The workspace

```text
┌──────────────┬───────────────────────┬──────────────────────────────┬─────────────────────────┐
│ bacnetio     │ ▾ BACnet/IP 192.168…  │ Search in DDC4000_#21  [CSV] │ 5101  Sollwert Aktuell  │
│ [HOOC TAP0 ▾]│   ▾ DDC4000_#21  ●1 ●1│ Name        Type  Value State│ 0 °C   Fault  Alarm     │
│ ● 3 devices  │     ▸ SY_Datsi.01   4 │ 5101 Soll…  AV    0 °C  Fault│ Priority array          │
│ [Search all] │     ▸ S337.01   ●1 12 │ 5102 Ist…   AV   21.4 °C     │ [1][2][3][4][5][6][7][8]│
│──────────────│     ▸ By type     170 │ DV01 Pumpe  BV    active     │ [9]…           [16]     │
│ Workspace  3 │   ▸ AHU-01        ●1  │                              │ Write 40 at 8 [Write]   │
│ Problems   4 │   ▾ Router → net 2    │                              │ ⚠ priority 5 would win  │
│ Recording    │       VAV-12  offline │                              │ Details, properties,    │
│ Held       1 │                       │                              │ trend, record           │
│ Settings     │                       │                              │                         │
├──────────────┴───────────────────────┴──────────────────────────────┴─────────────────────────┤
│ ● Reading values of S337.01 (12 of 12)                                         1 held by you │
└───────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Tree** (left of the work area): network → device → module (from names or Structured View) and
  "By type" with counts, filled during the search (object lists only, concept-explorer section 2.3).
  Faults and overrides bubble up as red and orange marks, so a closed tree shows where the trouble
  is. Devices behind routers sit under their router and network number; silent ones say "offline".
- **Table** (middle): the points of the node chosen in the tree, read when the node opens, with
  search, filters and CSV of that node. Choosing a device shows its facts above the table.
- **Details** (right): the chosen point (section 4), or the chosen device's facts, clock and
  reference.
- **Phone**: the table with its groups stands in for the tree, with "← Devices" as the way back; the details cover the table. Below 1200 px the details slide over the table too.

## 3. Site problems

A view in the sidebar: everything faulty, in alarm, overridden, out of service, held by you or
offline, across all devices that were read, with the device and the point, newest first. A click
opens the point in the workspace. Its count stands in the sidebar, red when anything is faulty.
Points not read yet are read in the background after the search, so the list fills without opening
every device.

## 4. The priority array as the centre of a point

- All 16 slots at once, each with its value and who usually holds it (life safety, critical
  equipment, minimum on/off, manual operator …), yours marked, the winning one filled, and the
  default below.
- **Warning before a write** that would not act: when a higher priority (lower number) holds a
  value, the write asks first ("Priority 5 holds 45 % and is higher than 8: your value would not
  act. Write anyway / Cancel"). The same for releasing a slot that does not win.
- Write and release stay behind the session unlock, with the log entry as today.

## 5. Leaving check

What you still hold stays visible: count in the sidebar ("Held by you") and the status bar, a view
listing each value with "Release". Closing the page or choosing "Leave the site" asks first and
offers "Release all and leave". After a restart the same list comes from the write log (already
true today), with the device found again by its address.

## 6. Search across the site

The search field in the sidebar (`/`) finds points by name, description and object across all
devices read so far, and devices by name, ID and address. Results list device and point; a click
opens it in the workspace. The table's own search stays for the chosen node.

## 7. Functions and where they live on

Every function of concept section 3.3 keeps a place. Pages that go away: *Devices* and *Data
points* (into the workspace); *Overview* (into the workspace's empty state and the sidebar counts);
*Log* becomes *Held by you* with the full log below.

| Function today | Place today | Place in the workspace |
| --- | --- | --- |
| Choose the adapter, search settings (*Change*) | sidebar | sidebar, unchanged |
| Search, live devices, routers, diagnosis notes | devices page | tree fills live; notes above the tree, also in *Site problems* |
| Device by IP | devices toolbar | tree toolbar, *Device by IP …* |
| Device list CSV | devices toolbar | tree toolbar, *CSV* |
| Device facts, set the clock | device details | details when a device is chosen in the tree |
| Read points with fallbacks | data points page | when a tree node opens, in the background after the search |
| Grouping by module or kind | data points list | tree levels below the device |
| Search and filters (faults, overrides, changes) | data points toolbar | table toolbar; site-wide in the sidebar search and *Site problems* |
| Multi-state texts | list, details, write | unchanged |
| Read again, point CSV, reference save and compare | data points toolbar | table toolbar of the device node |
| Live value, priority array, write, release, all properties | point details | details, priority array as in section 4 |
| Trend log chart and CSV | point details | details |
| Recording (add, interval, start, stop, remove, CSV) | details, recording page | details, *Recording* view |
| Values held, release, log | log page, status bar | *Held by you* view, sidebar and status bar counts, leaving check |
| Warning before leaving | browser | browser and *Leave the site* (section 5) |
| Overview with next step | overview page | empty workspace: one sentence and *Find devices* |
| Language, theme, keys | settings | settings, unchanged |
| A reload keeps the place | browser | also keeps the chosen tree node |
| Last search and points read after a restart | devices, data points | tree and table, marked as remembered |
| Object count and progress from the search | devices list | tree, beside each device |
| Request log of a reading | device details | details when a device is chosen in the tree |
| Active alarms of a device | device details | details of the device, also in *Site problems* |
| Point list as EDE and as CSV with priorities | data points toolbar | table toolbar of the device node |
| EDE file of all devices | devices toolbar | tree toolbar |
| Only pages of this bacnetio may use the API | server | unchanged |
| Layout: nothing clipped in any language, width or theme | every page | every view, same test |
| Command line: `scan`, `read`, `write`, `release`, `interfaces` | terminal | unchanged |
| Android app with the same UI | phone | three steps with "back" (section 2) |

Each row gets its test before the old page goes (AGENTS.md, "never lose a function").

## 8. Order of building (after approval)

1. Tree with devices and object lists from the search (concept-explorer step C), table and details
   beside it; the devices and points pages stay until the workspace has every function.
2. Priority array with the warning before a write.
3. *Site problems* and the site-wide search.
4. *Held by you* with the leaving check; then the old pages go, row by row of section 7.
