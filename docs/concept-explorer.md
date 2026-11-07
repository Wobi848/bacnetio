# Concept: the best BACnet browser

Status: proposal, 1.11.2026. Follows the first field test (Kieback&Peter DDC4000 over a HOOC VPN)
and the maintainer's goal: bacnetio has to beat every BACnet browser there is.

## 1. What the field test showed

| Seen | Cause |
| --- | --- |
| The K&P BACnet Browser lists devices at once, bacnetio after ~10 s | bacnetio waits for Who-Is, then sweeps, then reads names, and shows nothing before the end |
| Reading 170 points takes long | requests too large for the device run into timeouts (2 s each, twice) before bacnetio makes them smaller |
| Trend log and whole object list fail with "abort reason=10" | same: answers that need segmentation do not arrive, bacnetio waits for the timeout |
| bacnetio heard nothing at all at first | the K&P service holds 47808 on the adapter's address (fixed: bacnetio now moves to another port) |
| One long list of 170 points | no tree; grouping guessed from names only |

## 2. What the best browser does

Measured against the K&P BACnet Browser, YABE, CAS BACnet Explorer and the Contemporary Controls
discovery tool:

1. **First device on screen within a second.** Every I-Am appears as it arrives. The direct sweep
   (for firewalls) runs at the same time, not afterwards. Names are read for each device the moment
   it is found.
2. **Never wait for a timeout that could be avoided.** I-Am already says how large an answer the
   device takes (max APDU) and whether it segments. bacnetio plans every request from that: batch
   sizes for ReadPropertyMultiple, object list in one piece or in parts, trend log page size. A
   request that still fails is made smaller at once. Timeouts follow the measured round trip per
   device (fast on site, patient over a VPN) instead of a fixed 2 s.
3. **Everything streams, values only where you look.** The search also reads each device's object
   list, which is cheap (identifiers only), so the tree shows "Analog values (34)" right after the
   search, as the K&P browser does. Names and values are read when a branch opens, for that branch
   first, the rest quietly behind it. Nothing blocks: you click into a device while the search
   still runs.
4. **A tree that scales.** Network → device → view → object. Views:
   - *Plant* from Structured View objects where the device has them (as K&P's "system view"),
     else from path-like object names (`/21/000/09/11/S337.01/5101`);
   - *Types*: analog inputs, binary values, …, with counts;
   - faults, alarms and overrides bubble up to the device and the network, so a collapsed tree still
     shows where the trouble is.
   Rendered virtually, so 1000 devices and 50 000 objects stay smooth.
5. **Says why.** A request log with timing per device (what was asked, how large, how long, what
   came back) and the device's limits in plain words. When something is slow, the log shows where.
6. **Remembers the site.** Devices and object lists are kept per site on disk, so the next visit
   shows the tree at once and only refreshes values.
7. **What technicians need beyond browsing:** watch list across devices, recording (exists),
   writing with the priority array (exists), reference compare (exists), EDE export (the trade's
   exchange format), active alarms per device (GetEventInformation), and later a packet console.

## 2b. Functions seen in the K&P BACnet Browser

The maintainer shared the manual of the BACnet Browser Kieback&Peter ships (a rebranded third-party
product, edition 2022-11-22). Its functions, judged for bacnetio. Only the function is taken, not
text or design.

| Function there | For bacnetio | When |
| --- | --- | --- |
| Tree views: by device and object type; by object name split at separators across devices; one list of all objects by name | Tree with module and type views (concept-redesign); a site-wide list by name is the site search | step C |
| Known devices kept in a cache, *New search* only on request | Site memory: devices and object lists kept per site, refreshed on demand | step D |
| Add a device by address, also on another network (network number and MAC, MS/TP address) | Extend *Device by IP* with network number and MAC for devices behind routers | step C |
| All properties of an object with data type, how each is updated (polling every 10 s or COV), changed values highlighted, update counter | Properties table with data type and live updates; highlight changes | step C |
| Edit any writable property inline, not only the present value | Write other properties (out of service, COV increment, description, setpoints of limits) behind the session unlock, logged | after C, with a concept |
| Present values of a device with status flags and the winning priority | Points table gets a column "priority now" from the priority array | step C |
| Calendar view of schedules and calendars (day, week, month, year) | Show Schedule and Calendar objects as week and exception view, read only first | later |
| Several panes side by side, pinned | Compare two devices or points side by side | later |
| Start a Wireshark capture with BACnet filters | Request log exists; add export of bacnetio's own traffic as pcap for Wireshark | later |
| EDE export for one device, several or the whole site, header fields, XLS or CSV, UTF-8 | EDE exists per device; add whole site, project name, state text file | step D |
| Simple and advanced filters (device, status, event handling), grouping | Site search and *Site problems* (concept-redesign) | step C |
| Restore points of devices and objects, download and restore | Site memory export and import as file | step D |
| User address schemes: regular expressions that group object names (plant keys) | Optional own grouping pattern per site, beside the automatic path grouping | later |
| Mass operations: one property to many objects | Behind the unlock, with a list of what changes and the log; risky, so only with a concept | later |
| CSV/XLS of properties and present values, priority arrays with only the set slots | CSV gains the winning priority and the set slots | step D |
| BBMDs recognised, their BDT and FDT shown and edited | Show which devices are BBMDs and read their BDT/FDT; editing only with a concept | later |
| PDF of the current view | Print layout of a device's points, so the browser's "print to PDF" gives a clean report | later |
| BACnet Backup & Restore of device configurations | Possible later behind the unlock; not a browser's core | later or never |
| Values pushed by unconfirmed COV without subscription | Show such values as live too | step C |
| Own device ID and name of the client changeable | Not needed: bacnetio has no device object on the network. It only asks, answers no Who-Is and rejects requests to it, so there is no ID to clash with another tool | never |
| Users, roles, licence, tray monitor, Windows service, installer | Not needed: bacnetio is one local program without installation; the session unlock guards writing | never |
| MS/TP over an RS-485 adapter | Its own connection type ([concept-mstp.md](concept-mstp.md)) | built |
| Modbus devices | Out of scope (concept section 4) | never for now |

## 2c. Search as built: objects within seconds

The maintainer's yardstick: the K&P browser knows a site's devices and objects within 10 s. Measured
on the simulators (a controller of 181 objects and one of 19, on a /24 of otherwise unused addresses,
over a virtual network, 1.11.2026): devices, names and object lists show after 0.07 s, the search
ends after 6 s. The real site is on the field checklist.

The sweep used to ask all 253 addresses at once. A packet to an address nobody holds waits about 3 s
for ARP, and on a /24 those filled the send buffer of the one socket, so the names and lists of the
devices already found waited too (3 to 6 s, and 10 s for the whole search). A smaller subnet never
showed this. Hence 64 addresses at a time and 1.5 s per address.

## 3. Order of building

| Step | Content | Done when |
| --- | --- | --- |
| A | Live search: devices stream in, sweep parallel to Who-Is, names read per device as found | a /24 with one device shows it in under a second |
| B | Request planning from I-Am limits, adaptive timeouts, background reading with progress, request log | the DDC4000 over VPN reads without a single timeout |

**Step B as built (1.11.2026):** the limits come from the device object (max-apdu-length-accepted,
segmentation-supported) rather than I-Am, because devices found by asking directly have no I-Am
and the client's placeholders must not drive sizes. Object list parts, objects per
ReadPropertyMultiple and trend pages follow them (`src/limits.rs`); a request that fails is made
smaller at once. Reading runs in the background with progress and a request log per device.
**Adaptive timeouts are not built:** rusty-bacnet takes one timeout per client, not per device or
request; the request log shows the round trips, so the field test can tell whether it is needed.
Against a simulator without segmentation and 400 objects no request fails; the DDC4000 over VPN
is on the field checklist.
| C | Explorer tree replacing the device and point pages; plant view from names and Structured View | 170 points of the DDC appear grouped while they are read |
| D | Site memory, watch list, EDE export, alarms | — |

Each step keeps every existing function; the function list in concept section 3 moves with it.
