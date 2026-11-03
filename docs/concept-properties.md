# Concept: the properties of an object, readable

The maintainer finds the *Properties* list of a point plain (2 November 2026): "die Properties
könnten wir noch schöner darstellen". Mockup: [mockup/properties.html](mockup/properties.html).
Built only after the maintainer has seen it.

## What is wrong now

- Some 30 rows in one block, without order a technician recognises.
- The BACnet names wrap in the middle (`event-detection-` / `enable`) and are English in every
  language.
- Every row carries its data type (`real`, `text`, `enumerated`): noise for the daily look.
- Bit strings as digits (`00`, `000`, `111`), enumerations as bare numbers (`notify-type 0`),
  times without unit (`time-delay 0`).
- Structured values say only `(structured value)`; empty text lists show as `, ,`.
- An empty description shows nothing but its type.

## Then

**Sections** with the small headings the details already use, in this order; within each the
order is fixed, not the device's:

| Section | Properties |
|---|---|
| Object | name, description, type and instance, device and address, profile |
| Value | present value, status flags, reliability, out of service, COV increment, units, priority and relinquish default (outputs), polarity, state texts |
| Alarm limits | high and low limit, deadband, limit enable, time delay and time delay normal |
| Alarming | event state, event enable, acked transitions, notify type, notification class, event time stamps, message texts, event detection enable |
| Other | everything not named above, closed, with its count |

The device object gets its own sections (*Device*: vendor, model, firmware, protocol revision;
*Communication*: APDU timeout, retries, segmentation, max APDU; *Time*: local time and date, UTC
offset). Properties the device does not have are left out, so a section without any is not
shown.

**Plain words.** Each property has a name in the interface's language (*High limit*, *Obergrenze*).
A switch *Plain · BACnet* above the list shows the BACnet name and data type under each name, for
who wants to compare with the standard or another tool. The search above the list finds a
property by either name.

**Values by their meaning:**

- Yes and No instead of `true` and `false`.
- Units also where BACnet leaves them implicit: time delay in seconds, APDU timeout in ms.
- Status flags: one green *Normal* while no flag is set; otherwise each set flag as an orange chip
  (*Overridden*, *Out of service*), *In alarm* and *Fault* red.
- Event enable and acked transitions: the transitions that are set, by name; *All acknowledged*
  in green, *nothing switched on* in grey.
- Limit enable: next to each limit, *not monitored* when its bit is off.
- Notify type, event state, reliability and the like by name; *No fault detected* and *Normal*
  green.
- Event time stamps: one line per transition with its date and time, *never* for an unset stamp.
- Text lists one line each; empty ones *empty*, in grey italics; a long name breaks after its
  slashes.
- What still cannot be shown in words keeps its raw form under the BACnet switch, never
  `(structured value)` in the plain view.

**Unchanged:** read every 10 s, a changed value marked; *Change* on writable rows once unlocked,
the unlock offered above the list; every property that is shown now stays reachable, the rare
ones under *Other*.

## Built

Built on 2 November as described, with two differences: the device object keeps its own facts in
the device details (they never showed this list), so it has no sections here; and structures the
server cannot put into words yet (an event algorithm inhibit reference) say *composite, see
BACnet* in the plain view.

## Where the work lies

- The server delivers bit strings with their bit names and structured values (time stamps,
  references) as parts instead of a placeholder; the shape goes in `docs/api.md`.
- The web groups, names and formats; the names come in all five languages.
- Tests: an analog input, a binary output, the device object; plain and BACnet view; a changed
  value marked; a write after unlock.
