# Concept: KBOB check

The maintainer asked on 1 November 2026 for a check of a plant against the Swiss KBOB
recommendation on BACnet; on 5 November the public documents were found and the work was asked
for. It comes as the first **plugin** (`docs/concept-plugins.md`): installed apart from
bacnetio, as the maintainer wants.

Built on 5 November 2026: `plugins/kbob/` (rules from C1 in `rules.js`, the check in `check.js`,
the page), built into one file by `plugins/build.mjs`.

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
| 6 | Analog objects carry a unit; no-units is one of the BACnetEngineeringUnits and counts | Units | 5.3.4 |
| 7 | Binary and multi-state inputs and values carry state texts (outputs are not named there) | Active_Text, Inactive_Text, State_Text | 5.3.6 |
| 8 | Objects that report events name a notification class | Event_Enable, Notification_Class | 5.3.5 |
| 9 | The services behind the profile's server BIBBs (-B) are supported, where one service stands for one BIBB | Device: Protocol_Services_Supported | 6, C1.3 |
| 10 | Notification classes are KBOB classes (NC1–4, 32–35 … 224–225), with Ack_Required and priorities as table 8 gives | Notification Class objects | 6.2, tables 7 and 8 |
| 11 | Object names follow the AKS pattern NNNN_AAX_ANN_ANNN_AN, and their FUNKTION fits the object type (AI M/Z, AO Y, AV W/Z/M, BI D, BO S, BV H, MI D, MO S, MV H, TL T, SCHED U, CAL K, LP R, NC N, from the letters of the guide's table 2 and the object types of the functions in 7.2; Event Enrollment, Program, Event Log, Positive Integer Value and File have no letter there and are not checked); only when chosen, since the guide gives it as an example and the owner decides | Object_Name | guide 3.2.2, table 2; recommendation 7.2 |

## Checked against the sources

5 November 2026, each by machine where the documents allow it:

- the properties per object type and profile (`rules.js`, from C1.2) against appendix A of the
  recommendation: the same for all 20 types and both profiles;
- the BIBBs (C1.3) against the tables of chapter 6: the same for all 97;
- the object types (C1.1) against table 4: the same but for Multi-state Input, mandatory for AS-CH
  in table 4 and conditional in C1.1; table 4 counts;
- the notification classes and their acknowledgement and priorities against tables 7 and 8;
- the bit positions of Protocol_Services_Supported against the BACnet library's enumeration.

Rules 10 and 11 were added on 5 November 2026 from the maintainer's own GA tool, which already
checked EDE files for them; checked against the recommendation (tables 7 and 8) and the guide
(3.2.2) before they were taken over.

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

The aim: search the whole network, check every device, and see in bacnetio's own lists what does
not meet KBOB.

The plugin's page, in the sidebar once it is installed and on:

- The devices of the search or the open site, each with its profile (AS-CH, MBE-CH) and
  **Check against KBOB**; above them **Check all devices**, one after the other.
- A device's result: the rules as met, not met, not checkable, with what is missing, folded per
  rule.
- **Export** the report as CSV, one row per device and rule, with the source of each rule.
- The results are kept in the plugin's store, so they survive a restart.
- Every rule not met becomes a **mark** (plugin API): the device in the device list and the tree,
  the object in the point list, with the rule as text ("KBOB: no unit (5.3.4)"). With the filter
  *Marked by KBOB check* the point list shows only what has to be fixed.

## Code

- `plugins/kbob/`: the rules table (from C1, V2.0), the check and its page. The check takes what
  was read and gives the report, without I/O; tests feed it devices that meet and break each rule.
- It reads only through the plugin API, version 1.
- The simulated plant gets one device that meets AS-CH and one that does not.
- Texts in every language the interface has.

## Decided

5 November 2026, with the maintainer: a plugin; every device checked as AS-CH unless the user picks
MBE-CH; rule 4 on every object, since the report must be complete; rules 6 and 7 count as not met,
rule 8 only where an object reports events.
