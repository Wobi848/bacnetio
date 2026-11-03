# Concept: devices that share a device ID

Field visit 3 November 2026: seven Danfoss NovoCon behind a router on MS/TP network 132 all
answered as device 4194303. The maintainer gave them new IDs one by one. Since then (branch
`duplicate-ids`) a search starts afresh and a write goes to the address the details show
(`scan::aim_at`). Everything else still goes by the ID alone. This concept is about the rest.
It is to be shown to the maintainer before it is built.

## Why it matters

A device ID must be unique on a BACnet internetwork, and bacnetio keeps saying so. But a plant
with duplicates is exactly where a technician needs the browser: to see which device is which,
read each one and give it a new ID. Today:

- the client keeps one address per ID, the first it heard;
- the web API names a device by its ID (`/api/devices/{instance}/…`), and so do the list, the
  tree, the readings, the watch list, the recording, the write log and the site memory;
- so of seven NovoCons the list shows seven rows, but every one opens the same device, and its
  values may come from another than the one whose MAC the details show.

## Then

**A device key.** Every device in a search gets a key: its ID as text where the ID is unique
(`2796214`), and the ID with its address where it is not: `4194303@132:03` behind a router
(network and MAC), `4194303@192.168.10.40` on the local network (with `:port` when it is not
47808). The key is what the interface and the API name a device by. A plain ID keeps working
everywhere it works now, so links, stored data and scripts do not break.

**One device at a time per shared ID.** The client still knows one address per ID. Before each
request to a duplicate the server aims the client at the device meant (`aim_at`) and holds a lock
for that ID until the answer is in, so two requests to two NovoCons never cross. Duplicates are
read one after the other; unique devices in parallel as now.

**Everything that names a device takes the key:** the device list and the tree (each NovoCon
opens its own points), the reading and its request log, the live values, the details and their
writes, the watch list, the recording, the write log (*held by you* and *release*), the
references, the site memory and the exports. A COV report names its device by ID and comes from an
address; the server matches it by both.

**After a new ID** the key of that device becomes its new ID at once, and what was stored under the
old key (watch list, recording, held values) moves with it. The duplicate warning goes when the
last duplicate has gone.

**Unchanged:** the yellow note that two devices share an ID, with their addresses; the advice to
fix it on the plant. A key with an address is only a way to work with the plant until then.

## Never lose a function

Each function that names a device is checked with a duplicate pair in the simulators, beside the
tests it has now: open, read, live values, write, release, watch, record, export, site memory,
COV.

## Where the work lies

- Server: the key and its parsing, the lock per shared ID, every handler and store that names a
  device. Stored files take the key in a new field and still load without it.
- Web: `data-device` and every state map by device move to the key.
- Tests: two simulated devices with one ID on two ports (as in `tests/scan.rs`), through the API
  and in the browser.

## Decision

Not built. A shared ID is a fault in the plant that has to be fixed anyway, and most building
management systems cannot cope with it either. bacnetio reports it as an error (the scan
diagnosis names the ID and every address) and helps to fix it: find the devices and give each a
new ID. This concept stays as the plan in case working with duplicates is ever needed.
