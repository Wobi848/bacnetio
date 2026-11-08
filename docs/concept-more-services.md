# More of what devices keep: multi-point logs, event logs, files, stopping communication

Status: proposal, 08.11.2026, branch `concept-more-services`.

## Why

A review of the whole program on 08.11.2026 ("es wird Zeit das du alles durch checkst") compared
bacnetio with what a technician expects of a BACnet browser. Most of it is there: reading, writing at
a priority, COV, the device's own Trend Logs, alarms and events, schedules, Structured View, BBMD
tables, routers, MS/TP. What is missing:

1. **Trend Log Multiple** (object type 27). Many controllers log several points in one object, not one
   Trend Log per point. bacnetio shows such an object only with its properties; its records are not
   read. Today only Trend Log (type 20) is read (`src/trend.rs`).
2. **Event Log** (type 25). The device's own list of past alarms and events, with time stamps. A
   technician looks there first when the plant "did something last night". Not read today.
3. **File objects** (type 10). Programs, parameter sets and logs a device keeps as files. Reading
   them (AtomicReadFile) saves a device's programming before a change. Not read today.
4. **Stopping communication.** bacnetio sends DeviceCommunicationControl with DISABLE. ASHRAE
   135-2020 withdrew DISABLE: devices after that revision refuse it (`service-request-denied`), as the
   simulators do. What is left is DISABLE_INITIATION: the device stops sending by itself (I-Am, COV,
   alarms) but still answers. Today *Stop communication* fails on such a device, and the refusal
   does not say why.

Also missing, but less important: WritePropertyMultiple (several values in one request; *Change
many* writes one by one, which works everywhere) and COV per property (SubscribeCOVProperty; the
present value by COV covers what bacnetio shows).

The BACnet library bacnetio uses already has every service needed (`read_range`, `read_log_page`,
`atomic_read_file`, `device_communication_control`), so each point is work in bacnetio only.

## Proposal

### 1. Trend Log Multiple

Chosen like a Trend Log: the details show its records as a chart and a table, one line per logged
point, with *Save as CSV* and Excel. The points' names come from its Log_Device_Object_Property
list, read once. Records are read with ReadRange page by page, as for a Trend Log, with the same
limit of 20 000 records.

### 2. Event Log

Chosen in the point list, the details show its records newest first: time, object, from and to which
state, and the message text where the device gives one. *Save as CSV* and Excel. Nothing is
acknowledged from here; that stays with the alarms of the device.

### 3. File objects

Chosen in the point list, the details show size, modified date and whether it is read-only, and a
button *Save file* that reads the whole file with AtomicReadFile (stream or records, as the object
says) and saves it under the object's name. Reading only: writing a file back to a device changes
its programming and is left to the vendor's tool.

### 4. Stopping communication

The confirmation in the device commands offers two ways, each with what it does:

- *Stop answering* (DISABLE): devices before ASHRAE 135-2020 only.
- *Stop sending by itself* (DISABLE_INITIATION): every device. It still answers bacnetio.

A device that refuses DISABLE gets the reason in words: "the device no longer takes this stop
(withdrawn in ASHRAE 135-2020): stop its sending instead". A stop of either kind without a time is
held until resumed, as built on 08.11.2026 (branch `held-communication`).

## Writing to plants

Points 1 to 3 only read. Point 4 writes, as today: behind the session unlock, with the confirmation
naming the kind of stop, logged with its kind and whether it has an end, and held when it has none.

## Functions and where they live on

| Function | Today | Then | Test |
|---|---|---|---|
| Trend Log records | details of a Trend Log | unchanged | `tests/trend.rs` |
| Trend Log Multiple records | properties only | details: chart, table, CSV, Excel | new, simulator with a TLM |
| Event Log | properties only | details: list, CSV, Excel | new, simulator with an Event Log |
| File object | properties only | details: facts and *Save file* | new, simulator with a file |
| Stop communication | one kind (DISABLE) | two kinds, the refusal in words | `tests/ui/22-commands` |

Nothing moves; `docs/concept.md` section 3.3 gets the three new rows.

## Compatibility

New routes only; the log's entries keep their fields. A stop entry's text names its kind
(`communication off`, `sending off`), and logs written before load as they are. `CHANGELOG.md` names
each point when it is built.

## Order of building

1. Stopping communication in two kinds, as it fails on new devices today.
2. Trend Log Multiple.
3. Event Log.
4. File objects.

Each step with its simulator, test, texts in all languages, `docs/api.md` and help text.

## Open for the decision

1. Build all four, in this order? Recommendation: yes; 1 first, as it fails on new devices today.
2. File objects: reading only, never writing back? Recommendation: reading only.
3. WritePropertyMultiple and COV per property: leave out for now? Recommendation: yes, until a plant
   needs them.
