# Concept: KBOB check

The maintainer asked on 1 November 2026 for a check of a plant against the Swiss KBOB
recommendation on BACnet; on 5 November the public documents were found and the work was asked
for. It comes as an *extension*: off by default, switched on under *Settings* (quality plan,
"Basics first, then extras").

## Sources

KBOB, *Gebäudetechnik und Gebäudeautomation*,
https://www.kbob.admin.ch/de/gebaeudetechnik-und-gebaeudeautomation:

- *Empfehlung BACnet Anwendung*, V2.0, July 2021: the rules (chapters 4 to 7, appendix A).
- *Leitfaden BACnet Anwendung*, V2.0: how they are meant.
- Appendix C1, *Checkliste zur Kontrolle von BACnet Vorgaben* (XLSX): the same rules as a
  checklist, per profile: C1.1 object types, C1.2 properties per object type, C1.3 BIBBs.

The documents are not kept in the repo. bacnetio carries its own table of the rules, written from
C1 and naming the version (V2.0, 2021), so a later version can be added beside it.

## Profiles

KBOB tells two profiles apart: **AS-CH** for an automation station and **MBE-CH** for a
management and operator workstation. bacnetio looks at devices in the field, so a device is checked
as AS-CH unless the user picks MBE-CH for it.

## What is checked, by reading only

bacnetio never writes for this check. Each rule names its place in the recommendation or in C1.

| # | Rule | Read from | Source |
|---|---|---|---|
| 1 | BACnet protocol version 1, revision 14 or newer | Device: Protocol_Version, Protocol_Revision | 4.1 |
| 2 | Every object type the profile requires is supported | Device: Protocol_Object_Types_Supported | 5.2.1, C1.1 |
| 3 | No proprietary object types (number 128 or above) | Device: Object_List | 4.4 |
| 4 | Each object has the properties its type requires in the profile | each object: Property_List, else all properties | 5.3, appendix A, C1.2 |
| 5 | Every object has a name, and no name occurs twice in the site | Object_Name of all devices read | 5.3.3 |
| 6 | Analog objects carry a unit | Units | 5.3.4 |
| 7 | Binary and multi-state objects carry state texts | Active_Text, Inactive_Text, State_Text | 5.3.6 |
| 8 | Objects that report events name a notification class | Event_Enable, Notification_Class | 5.3.5 |
| 9 | The services behind the profile's server BIBBs (-B) are supported, where one service stands for one BIBB | Device: Protocol_Services_Supported | 6, C1.3 |

Each rule ends as **met**, **not met** (with what is missing) or **not checkable** (the device did
not answer, or the rule needs something bacnetio cannot read).

## What is not checked

Said in the report, so nobody takes the check for more than it is:

- whether a property can be written (*W* in C1.2), dynamic creation and deletion (DC/DD): would
  need writing to the plant;
- the minimum string lengths (5.3.1): would need writing;
- client BIBBs (-A), alarm and trend behaviour, the building automation functions of chapter 7;
- the owner's naming concept, reserve objects, BTL certification.

## In the interface

- **Settings → Extensions:** *KBOB check*, off. Off, nothing of it shows.
- **Device details:** a section *KBOB check* with the profile (AS-CH, MBE-CH) and a button
  *Check against KBOB*. The result shows as a list of the rules: met, not met, not checkable, with
  what is missing, folded per rule.
- **Site:** *Check all devices against KBOB*, one after the other, as *Read all devices* does.
  A table of devices by rules met and not met.
- **Export:** the report as CSV, one row per device and rule, with the source of each rule.
- Reading goes through the same queue as the point list, so the details keep their precedence
  (`src/precedence.rs`).

## Code

- `src/kbob/`: the rules table (from C1, V2.0) and the check, without I/O: it takes what was read
  and gives the report. Tests feed it devices that meet and break each rule.
- Server: one endpoint to check a device, one for the site; the report kept with the site, so it
  survives a restart.
- Web: a module of its own, loaded only when the extension is on.
- The simulated plant gets one device that meets AS-CH and one that does not.
- New texts in every language (`docs/concept-languages.md`).

## Open, for the maintainer

1. AS-CH as the default for every device: right?
2. Rule 4 on every object (one Property_List per object, slow on MS/TP) or on one object per type?
   Proposal: every object, because the plant is checked once and the report must be complete.
3. Rules 6 to 8 as errors or as hints? Proposal: errors, since the recommendation says *must*,
   except 8, which applies only where events are used.
