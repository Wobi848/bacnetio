# Changelog

All notable changes for users of bacnetio.

## Unreleased

### Added

- The priority grid names every priority under its number, not only the ones holding a value. With the KBOB check installed, the names follow KBOB table 10 (7 manual on site, 8 manual from the MBE, 12 central functions, 14 time programs, 16 automatic mode; the rest free, by agreement with the owner), also in the choice of priority when writing. Plugins can name priorities through plugin API 1 (`priorities`).
- A simulated controller with commands at several priorities (`cargo run --example simulated_commands -- <IP>`).
- The first extension, *KBOB* (`kbob.bacnetio-plugin.html` beside the programs in each release): checks every device of a search against the Swiss KBOB recommendation *BACnet Anwendung* V2.0, by reading only. Rules from the recommendation, its guide and checklist C1 (protocol revision, object types, properties per object type, names, units, state texts, notification classes and KBOB classes NC1–NC225, commands only at the priorities of table 10 and schedules at 14, services), and on request object names after the AKS pattern of the guide, per device met, not met or not checkable, with what is missing; devices and objects are marked in the device list, the tree and the point list, *KBOB ✓* in green or *KBOB ✗* in red with what is missing; a report as CSV. Profile AS-CH or MBE-CH per device.
- Extensions (plugins): one file each, installed, switched and removed under *Settings → Extensions*. An extension gets a page in the sidebar, reads devices, objects and properties through bacnetio, and can mark devices and points in bacnetio's lists. It runs in a sandbox: it cannot write to the plant, reach bacnetio's server or the user's files. How to write one: `docs/plugins.md`.

### Changed

- The note of a new version is filled green, so it catches the eye.

### Fixed

- Releasing from *Held by you* first reads the slot from the device and releases only while it still holds what bacnetio wrote. If another system has written over it or released it since, bacnetio says so and takes nothing away. Each held value shows since when it is held.
- After an update the note no longer offers the version now running: the kept answer of the check counts only for the version that asked.

## v0.1.2 (05.11.2026)

### Changed

- A connected VPN is chosen before mobile data; Wi-Fi and a cable still come first.
- When only direct questions found devices through a VPN, the note names the VPN instead of the firewall.
- On a phone the head folds to one line with the site and its devices; a tap unfolds the adapter, *Change* and the site search. The tabs swipe in one line, and watch and record sit beside the value, so a point takes one line and the list gets most of the screen.
- In the point list, press on a watch or record switch and drag the mouse over other rows: they all take the same state, as in Blender. A plain click still switches one; touch screens scroll as before.
- The top of the device tree is a button, *All devices*, so the way back to the device list is easy to find.
- In the details *Properties* stands apart from *Watch* and *Record*, as a button that opens the properties below.
- Android: the back key steps back (details, device, device list, workspace). With no step left it asks whether to leave bacnetio running in the background or to stop it, and warns of values still held.
- Starting bacnetio a second time opens the one already running in the browser, instead of a second bacnetio that would write the same files and find fewer devices.

### Fixed

- A changed adapter address, a VPN that comes up or a cable plugged in shows without restarting bacnetio; the chosen adapter stays chosen, also under its new address.
- Recording: with nothing recorded the page looks like the empty watch list, without the darker background and the double indent.
- Recording: a click on the page no longer gets lost when it falls into the page's regular refresh.
- Android: when another app holds port 8081, bacnetio takes a free port and keeps it, instead of not starting or showing the other app's page.
- Recording: *Poll* or *COV* and *Remove* sit in one line under the name; the switch lost its double line at the top.
- On a phone the status line keeps what is running readable (*Reading values …*) and shortens the adapter instead.

## v0.1.1 (04.11.2026)

### Changed

- Android: the app icon is the new logo.
- On a phone the site problems show the device above, so a value stays beside its point, and the change log gives the point its own line, so the change itself is no longer cut short.

### Fixed

- Android: the status bar, the camera cutout, the navigation bar and the keyboard no longer cover the interface.
- On a phone the group heads of the point list (*Inputs*, …) no longer stop below the top and cover the rows scrolling under them.
- Mobile data (`ccmni`, `rmnet`, `wwan`, *Cellular*) is no longer chosen by itself before Wi-Fi; it stays in the list to choose.

## v0.1.0 (03.11.2026)

The first release.

### Changed

- A point you open, its properties and its live value go before the reading of the list: the reading pauses between two of its requests while they run. Behind a router to MS/TP, where one request at a time gets through, the details no longer take turns with the list.
- New logo: the emblem in the Windows program and the README, a simplified form of it in the browser tab and the sidebar, where the emblem is too fine.
- Buttons by weight: the page's next step stays filled in the accent colour, tools are neutral with a grey border and take the accent under the mouse.
- The properties of a point are grouped (*Object*, *Value*, *Alarm limits*, *Alarming*, the rest under *More properties*), named in the interface's language and shown by meaning: Yes and No, state and acknowledgements as chips, limits marked *not monitored* when switched off, the last alarm, fault and normal reports with their time, seconds on delays. A search finds a property by either name; *BACnet* switches to the standard's names, the data type and what the device sent, and the browser remembers the choice.
- Recording charts: time marks at round times (every 10 s, 5 min, 1 h, …) instead of only start and end; binary and multi-state points on their states (*inactive / active*, *state 1 …*) instead of numbers, also in the tooltip and the latest value; the comparison is lower (about 220 px for numbers, less for states).
- Site problems and the watch list are tables with the heads *Device · Name · Value · State*, sorted by a click like the point list.
- The sidebar names the network once: under the site's name when it has one; the line below the adapter shows only while the next search would look elsewhere. The site search says *Search the site* and shows its key `/`.
- From 1200 px window width the details of a point stand beside the list instead of covering it (before: from 1400 px). In a narrow list the views wrap onto a second line instead of being cut off, and the state stays in its row.
- The toolbar of the point list is shorter: object types are ticked in a menu *Object types* (before: a row of chips), and *More …* holds the exports and the device details beside the tools it had.
- The kept sites have a page of their own, reached from the top of the sidebar, instead of a card in the settings. Each site can be given a name (*Sample School*) with its network below; the sidebar shows the open one. Each kept site saves to a file on its own; opening, removing and loading from a file work as before.
- The object list: a device opens with values showing instead of closed groups; wider name column, the object ID stays when space is short; numbers line up whatever their unit; group names stay in view while scrolling; rows under a group are indented under its name.
- The details show *Watch*, *Record* and *Properties* as one row of toggles that show whether they are on, instead of three buttons below each other.
- Plant parts hold several points: where every point has a path of its own, as with K&P (`001/00/00/P.01/CAI/b`), the group is taken a level higher (`001/00/00`) and the rest of the path stays in the row's name.
- The counts above the list count what the list shows for the chosen plant part.
- Object counts leave out the device object, so the tree, the device list and the table agree.

### Added

- A note when a new version is out: once a day bacnetio asks GitHub for the newest release and shows *New version …* in the sidebar, linked to its download page. *Settings → Version* shows the running version, what the last check found, *Look now*, and a switch to turn the check off; then bacnetio does not go online.
- *Change device name* in the device commands, behind the unlock: the device list, the tree and the details show the new name at once. A device that refuses says so in words.
- Columns to resize in the device list, the point list, site problems and the watch list: drag the border right of a column head (a line shows under the mouse). The first column gives or takes the room; the browser remembers each column's width, also when the details open or close; a double-click returns to the automatic widths. Not in a list as narrow as a phone.
- Site problems and the watch list have the search of the point list (also `#123`, `type:AO`, `value:>50`, and the device's name), the states as views with their counts (*Alarm*, *Fault*, *Read error*, *Offline*, *Overridden*, *Out of service*), and a choice of one device; *Clear all filters* returns to everything.
- The properties API gives list entries one by one (`items`) and bit strings bit by bit (`bits`); notify types and other enumerations named by their property come by name, event time stamps as date and time.
- The borders between sidebar, tree, list and details can be dragged (or moved with the arrow keys when focused); the browser remembers the width, a double-click returns to the automatic width.
- The search of the point list knows `#2796214` (exact instance), `type:AO` (several with commas) and `value:>50` (also `<`, `<=`, `>=`, or a word such as `value:active`); plain words search name and description as before.
- Several object types can be chosen at once (before: one). What narrows the list shows as chips that can be removed one by one, or all at once.
- Views *Alarm*, *Out of service* and *Held by you* beside *Faults* and *Overridden*, shown while some point is in them.
- The recording measures each point by *Poll* (at the interval) or *COV* (every report of the device, with its own time); a device that refuses COV is polled instead, and the point says so. The CSV marks COV columns.
- A COV point's line goes on at its last value until now while recording, since the device reports only changes.
- Each measurement shows as a dot on the line, each one without answer as a red tick, and each point says when it was last measured, flashing once at every new measurement.
- A background to choose per theme in the settings: light *White*, *Grey* or *Warm*, dark *Navy*, *Graphite* or *Black*; each checked for contrast, the sidebar stays darker than the content.
- An accent colour to choose in the settings: blue, crimson, violet or teal; states keep their own colours.
- Each row of the list has two small buttons to watch and to record the point without opening its details; filled when on.
- The list says when its values were read, in orange after 5 minutes.
- Keys: `1` to `6` choose the areas, side by side on the keyboard (before: `0` to `5`); `f` searches the list; Home, End, Page Up and Page Down move through it.
- Integer Value, Positive Integer Value, Large Analog Value, CharacterString Value, Lighting Output and Binary Lighting Output can be written like other values.
- *Stop* in the status bar ends a running search or reading, also *Read all devices*. What was found or read stays, with a note about the rest.
- A priority another system wrote can be released: choose its slot, then *Release priority N*, after a question.
- The details explain *Overridden*: the device reports a local override, such as a hand switch, which the priorities do not show.
- Frequent refusals of a device in words, such as a wrong password or a value it does not allow to be changed, with the BACnet code in brackets.
- Changing a device ID tries the wildcard instance 4194303 when the device does not know the ID it answers under, as happens with IDs set beyond the 4194302 BACnet allows.

### Fixed

- The heads of the point list stand over their columns: the state column took its width from each row's content, so *Description* and the others drifted from the values below.
- The active alarms of a device no longer push its details away: their count in the heading, how many are not acknowledged, the first five and *Show all*. Each alarm by its name where the points were read, its state in words (*Off-normal*, *High limit*), and *Acknowledge* in place of *not acknowledged* where the device allows it.
- Site problems and the watch list keep their columns in line from row to row; *Remove* in the watch list has a column of its own instead of sticking out on the right.
- The device list shows network and MAC of a device behind a router (*network 132 · MAC 07*) instead of a cut-off *behind router 10.3…*; the router is in the tooltip. The column head *Objects* no longer runs into *Vendor*.
- Devices that share a device ID: a new search found a device under an ID where another had been before only after bacnetio was restarted, and a write, such as a new device ID, could reach another of them than the one shown in the details. Each search now starts afresh, and a write goes to the address the details show.
- The tooltips of the areas in the sidebar named the keys of before (0 to 4, settings 5) instead of 1 to 6.
- A search on another adapter or port failed with *a request is still running* once a search had been made: the background task that keeps devices reachable, and a running recording, held the old connection open.
- Devices stay reachable more than 10 minutes after the search. Before, every request to them then failed with "not in device table".
- The priorities in the details show a change by another system as soon as the value changes, not up to 20 s later.
- A priority counts as yours only while it still holds what you wrote; written over by another system, it no longer says *You*.
- A value another system has written over or released leaves *Held by you*, so *Release all* never takes away someone else's value.
- The recording says when it has not been started yet, with a *Start* button in the note, instead of waiting for a first value.
- The technical details of an input offer *Change properties*; before, there was no way to allow changes there.
- The large value in the details has no gaps around the decimal sign.
- The value field of the write form no longer runs under the priority, and a value stays on one line with its unit.
- Chart axes show as many decimals as it takes for their labels to differ.
- Recorded names keep their upper and lower case.
- A point renamed in the details is found by the list's search at once.
- The plant problems show values in the language of the page, such as *inaktiv* instead of *inactive*.
- Toggle buttons (All / Fault / Overridden, By type / Plant, language, theme) stay readable under the mouse.
- A long recording no longer breaks the chart.
- *Held by you* no longer breaks after a device command, an alarm acknowledgement or a schedule change.
- After a restart or opening a kept site, CSV and EDE exports, the device list and reference comparison work on the remembered data; opening a device no longer shows an error.
- A value you wrote no longer jumps back on screen when the device's last COV report was older than your write.
- Changing one time of a schedule day leaves the seconds and decimals of its other entries as they were.
- Reading a trend log stops after a limit if a device never reports the end.
- Entering a device's own ID says so, instead of claiming another device has it.

### Changed

- Setting the clock and acknowledging an alarm ask once more, also when changes are already allowed.
- Setting the clock says the time was sent: the device gives no answer to this request.

### Interface
- New interface after mbusio: sidebar with adapter, search settings and areas; page head with the
  one next step; lists with details beside them; status bar; light, dark and automatic theme;
  keys `0`–`5`, `/`, arrows, Enter, Escape.
- The interface takes the system's language (English or German) unless one is chosen in Settings.
- French, Italian and Spanish are translated (interface, notes, errors, Android). They are not in
  the menu until someone from the trade has reviewed them; `?lang=fr`, `?lang=it`, `?lang=es` open
  them. Dates follow the language's locale, numbers take a decimal comma in all but English.
- A reload keeps the area, the search result, the open device and the search settings.
- After a restart the last search and the points read show at once, marked as remembered, with
  *Search again* as the next step.
- Data points are grouped by module where controllers name objects as paths
  (`/21/000/09/11/S337.01/5101`), with the description beside the code. Groups show faults,
  overrides and objects out of service while closed.
- Where a device carries a plant view of its own (Structured View objects), the tree shows that
  view instead of the grouping by names: every branch with what hangs below it, views inside views
  indented, a branch's objects in the device's own order, and *Not in the plant view* for the
  objects no branch holds. Objects that a branch names on another device are counted and named as
  such. Nothing is written; the view is only read.
- The recording compares values: a chart per unit on one time axis, the last 5, 15 or 60 minutes or a
  span drawn with the pointer, and each value's lowest, highest and mean, share of time on, changes
  and missing answers. Recorded values can be cleared. Trend logs show the same figures.
- The tree switches between *By type* and *Plant*, and remembers the choice.
- The object list has description and ID in columns of their own, and marks what the search found.
  *Live values* keeps the rows on screen up to date. All groups open or close at once. *Save reference*
  and *Read priorities* moved into *More …*, so that the toolbar stays with what is used all the time.
- Device and point lists sort by any column; numbers and addresses sort as such, and empty cells stay
  last. Points also filter by object type.
- Multi-state values show the device's state texts, also as choices when writing.
- Every object type of the standard has a name in every language, so a meter panel shows
  *Accumulator* in the tree and in the type filter instead of the bare number 23; only objects
  with a value offer to be recorded.
- Setting a device's clock is in the device details and written to the log like every write.
- After a write or release, the list shows the new value at once instead of up to 2 s later.

- New look: one family of cool blue-greys with one blue accent, the current page filled in the
  sidebar, more room in lists and details, cards over the whole width. On the phone all areas stay
  in sight instead of behind a sideways scroll.

- One workspace instead of separate pages: a tree of the site (network, devices behind their router,
  plant parts and object types with marks for faults and overrides), the table of what is chosen in
  it, and the details beside. Old addresses (`#/overview`, `#/devices`, `#/points`) lead there.
- All 16 priorities of a point at once, with who holds what and what wins. A write or release that
  would not act asks first. A value being typed is no longer overwritten by the live update.
- *Site problems*: everything faulty, in alarm, overridden, out of service or offline across the
  devices read, with *Read all devices*.
- Search across the whole site in the sidebar (`/`).
- *Watch list*: points across devices with their values live, kept in this browser.
- *Held by you* replaces *Log* and shows the same, also objects put out of service until taken back, with *Leave the site*, which offers to release
  everything first.

- Technical details: every property with its data type, updated every 10 s with changes marked. Any
  property with a plain value can be changed after the unlock, logged like every write; the device
  decides whether it takes it.
- The technical details say in words what a device can do and who gets its alarms:
  *protocol-services-supported* and *protocol-object-types-supported* as names
  ("subscribe-cov, read-property, read-range") instead of a 60-character bit string, and a
  Notification Class's *recipient-list* as recipient, days, time window, confirmed or not,
  transitions and process instead of "(structured value)". A list cut after 20 entries says in the
  page's language how many are left.
- Device commands in the device details: warm and cold start, stopping and resuming communication,
  each after a warning, with an optional password that is sent but never stored, and logged.
- Alarms can be acknowledged in the device details. Devices that only know GetAlarmSummary show their
  alarms too. Alarm messages devices send to this computer show in *Site problems*.
- Schedule and calendar objects show their week, exceptions and dates in words. A day of the week can
  be changed after the unlock, logged.
- The search can ask the whole network who has an object of a name (Who-Has).
- A point's details choose how its value updates: by COV where the device offers it, or asked every
  2 s.
- *Read priorities* shows the priority that wins on each point.
- Each search is kept as a site of its own. Settings lists them to open again or remove, and a site can be saved
  to a file and loaded from one.
- A site file is checked before it is taken in: anything but numbers where numbers belong, a value,
  state, unit or state text that is not text, a search that is not finished, and more devices than
  a site has are refused, so a broken or hostile file cannot leave the page unusable. A file that
  is no JSON says so in a sentence in the page's language instead of showing the browser's parser
  message.
- A device's ID can be changed in its details, after the unlock and a clear warning, if the device
  allows it. IDs out of range or already taken are refused, and the change is logged.
- All messages for a wrong request now come in the page's language, including what the search
  settings, the address fields and the recording refuse; a test over the whole program keeps them
  complete.
- Messages about an answer that does not fit say it in words instead of Rust's own notation:
  "cannot read weekly-schedule" instead of `cannot read WEEKLY_SCHEDULE`, "the device reports the
  object count as the number 3, not as a whole number" instead of `Real(3.0)`, and the device's
  refusal by class and code in words.
- *Device by IP …* also reaches a device behind a router, with network, MAC and device ID.

### Search and reading
- Devices appear the moment they answer; the direct question to every address runs at the same
  time as Who-Is, and names are read at once.
- The search also reads each device's object list, so the device list shows how many objects a
  device has before anyone opens it, and opening it does not ask for the list again.
- Names and object lists no longer wait for the sweep of a large subnet. It asks 64 addresses at a
  time and gives each 1.5 s, which also shortens a search of a /24 from about 10 s to about 6 s. A
  device answering the direct question later than that is found only through Who-Is.
- Values a device reports unasked (unconfirmed COV) count as live.
- An object list read in one piece is checked against the count the device gives; if entries are
  missing, it is read in parts, so that no objects go missing unnoticed.
- A device that cannot send its object list in one piece is read in parts of as many entries as its
  largest answer holds (38 with a 480-byte APDU, up to 100) instead of one entry per request, which
  is much faster over a VPN.
- Before reading, bacnetio asks the device for its largest answer and whether it segments, and
  sizes every request to fit, instead of trying large requests and waiting for them to fail.
- Points appear while the device is read, with progress ("Values 64 of 170"); each device has a
  request log with the duration and outcome of every request (device details, *Show request log*).
- When another program (such as a BACnet browser running as a service) holds port 47808 on the
  adapter's address, bacnetio takes another port and says so; before, devices seemed silent.
- On Windows, virtual adapters (VirtualBox, Hyper-V, VPNs) are recognised by their description and
  no longer chosen by default; the adapter list shows the description.

- The device details read a BBMD's two tables on request: the broadcast distribution (which BBMDs
  it passes broadcasts to, and whether to each BBMD itself or to its subnet) and the devices
  registered with it, each with the lifetime asked for and what is left of it. A device that is no
  BBMD, or that does not give its tables, says so in words. The tables are only read; bacnetio
  never changes them.

### Exports and alarms
- Point list as EDE file (Engineering Data Exchange, layout 2.2) for management systems, per device
  or for all devices at once; devices not read yet are read first.
- EDE files carry the project name, whether an object type can be written and a reference to a state
  text file, which downloads beside them.
- Point list as CSV with the priority that wins now and every set slot, read from the device.
- The device details show the device's active alarms and faults (GetEventInformation).

### Safety
- Only pages served by bacnetio itself may use its API. A page elsewhere could have reached it
  through DNS rebinding and written to the plant.
