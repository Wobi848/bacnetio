# EDE files (layout 2.3)

Status: built on 08.11.2026, branch `ede-2-3`.

## Why

The maintainer asked whether the EDE export is right. bacnetio wrote layout 2.2 with 16 columns, gave
binary objects no state texts, put binary and multi-state texts with the same words into one row,
named the project after the network searched and the files `device-1001-ede.csv`. Checked with an
EDE checker and against the layout's description, these were the faults of the export itself; the
rest (long names, missing descriptions, names used twice) is in the plant's data and stays as the
devices give it.

## Source

EDE is no part of ASHRAE 135; it is the B.I.G.-EU's "Engineering Data Exchange Template for BACnet
Systems", version 2.3 (16.01.2017), downloadable from https://www.big-eu.org/resources/. Its English
description of layout 2.3 is quoted in full in the annex "600-10 Spécification EDE" of the CHUV
(V4.0, 23.04.2024, https://www.chuv.ch/fileadmin/sites/cits/documents/cit-directive_600_annexe_10.pdf).
The rules below are quoted from there, by the text, not from memory.

## The rules and how bacnetio follows them

| The layout says | bacnetio |
|---|---|
| Header *Project_Name*, *Version_Of_Reference_File*, *Time_Stamp_Of_Last_Change*, *Author_Of_Last_Change*, *Version_Of_Layout*: "valid for layout version 2.3" | `PROJECT_NAME` (the site's own name, else the network searched, or the device's name), `VERSION_OF_REFERENCEFILE` 1, the date, `bacnetio`, `2.3` |
| 17 columns, keyname to object-instance mandatory, the rest optional; the last one: "notification-class … the instance number of the notification_class object linked to the referenced object. If the object does not support intrinsic reporting, this field shall be empty." | The 17 columns in this order. Notification_Class is read of analog, binary, multi-state and loop objects; empty when the object has none |
| keyname: "the system wide unique name of the data object … In best case the keyname is identical to the object-name, but different names are allowed … where there are many devices with identical object-names on the same network" | `<device ID>_<object name>`, so a site file stays unique when devices use the same names |
| state text reference: "valid for binary and multi-state objects only" | Binary objects refer to their Inactive_Text and Active_Text, multi-state objects to their State_Text |
| State-Texts: "Multi-State- and Binary-Texts shall not be mixed under one reference number i.e. in the same row. The row contains therefore either the list "Inactive-Text, Active-Text" or "Text1, Text2, Text3,…"" | A row per distinct list and kind; the same words as binary and as multi-state texts get two rows |
| CSV file names: "Prefix: Name agreed by the involved parties (e.g. the project name)", separator `_`, suffix `EDE`, `StateTexts`, `ObjTypes`, `Units`, extension `.csv` | `<project>_EDE.csv` and `<project>_StateTexts.csv`; characters a file name cannot hold become `-` |
| settable: "The letter 'Y' means settable, the letter 'N' means not settable … If the field is empty, the client will assume that the value may be read and set." | `Y` for commandable object types, `N` otherwise: what the type allows; whether a value object really takes writes only the device knows |
| supports COV: "The letter 'Y' or an empty field indicates that the object supports COV." | Empty: knowing it would take a subscription per object |
| present-value-default, min/max-present-value, hi-limit, low-limit, vendor-specific-address: optional | Empty: each would take requests per object |
| Unit-Texts and Object-Types: lists of the codes used, needed for proprietary units and types | Not written: unit-code and object-type are the standard's numbers, which every EDE tool knows |

## Compatibility

- The file names change (CHANGELOG). The API routes stay; `GET /api/devices/{id}/points.ede-states.csv`
  and `GET /api/site.ede-states.csv` take `?project=` as the EDE file does.
- The point list in the API gains `binary_texts` and `notification_class`; site memory and site files
  without them keep loading.
- Reading a device asks two or three more properties of binary and alarming objects. With
  ReadPropertyMultiple that costs nothing worth noting; without it, one request each.

## Open

- Unit-Texts and Object-Types files, should a management system ask for them.
- The checks in the plant's data (unique names, lengths, descriptions) belong to a check plugin, not
  to the export.
