# Help in the program

Status: approved on 08.11.2026 (help in the program, German and English first), branch `help-concept`.

## Why

The maintainer asked whether bacnetio needs a wiki or help pages ("ob wir ein wiki oder helper seiten
erstellen sollten"), and what it is about: how to use the tool ("wie man es nutzt"). Today the only
description of how to work with bacnetio is `README.md` (Usage). It is written for GitHub, in English,
and lists what exists, not how to do a task. Someone on site does not read it, and the network there
is often closed.

Several tasks need steps that are not visible from the screen alone:

- a plant behind a VPN (subnet and adapter under *Search settings*, because Who-Is does not pass)
- a BBMD in another subnet
- MS/TP over a USB RS-485 adapter (listen first, then join, leave the trunk)
- writing and releasing a value (unlock, priority, *Held by you*, leaving the site)
- the EDE export for the whole site, the comparison of two devices, a reference to compare with later

## Proposal

### A help page in the program, no wiki

The help is part of the program, not a website:

- it works on site without internet
- it always matches the version installed, while a wiki describes the newest one
- it speaks the interface's language
- a guide can lead to the exact place in the program

A GitHub wiki would be a second copy of the README that drifts apart from the code. The README
(Usage) stays the short reference for GitHub and the command line.

### What the user sees

*Help* sits in the sidebar under *Settings* and also opens with the key `?`. On the left it lists the
guides, on the right it shows the chosen one:

```text
┌ Help ───────────────────────────────────────────────────────────────────┐
│ Getting started                │  Reach a plant behind a VPN             │
│   Find the devices             │                                         │
│   Read a device's points       │  Who-Is does not pass a VPN, so bacnetio │
│ On site                        │  asks every address of the plant's       │
│ ▸ Reach a plant behind a VPN   │  subnet directly.                        │
│   A BBMD in another subnet     │                                         │
│   MS/TP over USB RS-485        │  1. Connect the VPN.                     │
│ Working with values            │  2. Search settings → adapter: the VPN.  │
│   Write and release a value    │  3. Subnet: the plant's, e.g. …/24.      │
│   Record values                │  4. Search with these settings.          │
│   Compare with a reference     │                                         │
│ Exports                        │  [ Open the search settings ]            │
│   Excel, CSV, print, EDE       │                                         │
│   Capture for Wireshark        │  If nothing is found: …                  │
│ Sites and extensions           │                                         │
│ Keys                           │                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

On a phone the list comes first and the chosen guide below it, as in the other areas.

Each guide is written the same way:

- a sentence on what it is for
- the numbered steps, with the button names exactly as the interface shows them
- "If it does not work": the usual causes, taken from the diagnosis texts and `docs/field-test.md`
- one button to the place where the first step happens (*Open the search settings*, *Open Held by
  you* …); the guide only opens that place, never runs an action

The key overview moves from *Settings* into the help as its last guide. *Settings* keeps a button
that leads there.

### Reader

A technician who knows BACnet (device, object, priority, BBMD, COV) but not bacnetio. The guides
explain bacnetio, not BACnet. A BACnet term is explained only where bacnetio does something with it
that others do not, for example why the priority column appears only after reading.

### Where the texts live

There is one file per language, `web/help/<lang>.js`, beside `web/lang/`, so the long texts do not
swell the interface files. A guide consists of a key, a title, an introduction, steps, causes and a
target. The steps name buttons through the interface's own keys (`{btn:side.searchSettings}`), so a
renamed button changes in the help too.

A test checks four things:

- every language has the same guides and the same number of steps
- every named button key exists
- every target exists
- every guide is reachable from the list

## Writing to plants

None. The help only opens pages and panels; writing stays where it is, behind the unlock.

## Functions and where they live on

| Function | Today | Then | Test |
|---|---|---|---|
| Key overview | *Settings → Keys* | *Help → Keys*; *Settings* leads there with a button | browser test help |
| Everything else | unchanged | unchanged | — |

`docs/concept.md` section 3.3 gets the help page as a new entry.

## Compatibility

There is no stored data and no API. The key `?` is free so far. `CHANGELOG.md`: "Added: help in the
program, with guides for the usual tasks."

## Order of building

1. [x] Help page, navigation, `?` key, the key overview and the first three guides (find devices,
   VPN, write and release) in English and German, with the tests
2. [x] The other guides: read points, BBMD, MS/TP, watch list, recording, reference and comparison,
   files, capture, sites, extensions
3. [x] French, Italian and Spanish, as drafts like the rest of their texts until someone from the
   trade has reviewed them
4. [x] README (Usage) points to the help

## Decided

1. Help in the program, no wiki (08.11.2026).
2. German and English first (08.11.2026).
3. No explanations at the areas for now, as recommended; the help page comes first.

## Open

- The list of guides above: whether one is missing or not needed, once the maintainer has used the
  first ones.
