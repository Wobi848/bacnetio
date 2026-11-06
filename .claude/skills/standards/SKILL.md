---
name: standards
description: "Take rules from a standard, recommendation or checklist (KBOB BACnet Anwendung and its checklist C1, the BBL labelling directive, ASHRAE 135) into bacnetio or a plugin so that they are exactly right: transcribed by machine, checked against every other place the source states them, each with its chapter. Use whenever a check, a table of allowed values or a rule comes from such a document, when the maintainer says a rule must be correct (\"muss 100% stimmen\"), or asks where a rule comes from."
---

# Rules from standards, exactly

A check that reports a fault the source does not demand costs the tool its trust. Every rule must
be traceable to a sentence or table of the source, and nothing stricter than the source.

## Sources

- KBOB (public): `reference/kbob/` (not in Git, see `.git/info/exclude`): *Empfehlung BACnet
  Anwendung* V2.0 (rules, tables 4 and 7–9, chapter 6 BIBBs, 7.2 functions, appendix A properties),
  *Leitfaden* (3.2.2 object names, table 2 FUNKTION letters, 3.2.3 state texts), appendix C1
  (checklist: C1.1 object types, C1.2 properties, C1.3 BIBBs), T1/T2. Missing files: download from
  https://www.kbob.admin.ch/de/gebaeudetechnik-und-gebaeudeautomation.
- The maintainer's own GA tool (private; where it is: `CLAUDE.local.md`), which already checks EDE
  files against KBOB: AKS, KBOB functions, notification classes, state texts. Its code is the
  maintainer's; what goes into the public repo is the logic, not its explanatory texts.
- Search them before asking the maintainer anything a source answers.

## Steps

1. **Read the source, do not recall it.** `pdftotext -layout` for PDFs, the XLSX with Python's
   zipfile (no extra libraries). Symbols: ■ (U+25A0) or U+F06E mandatory, U+F06F conditional.
2. **Transcribe by machine** into a table in the code, with the source's numbers (`AI-06`, table,
   chapter) beside each entry; never type a table by hand.
3. **Compare with every other place** the same source states it, by machine: KBOB C1.2 against
   appendix A, C1.1 against table 4, C1.3 against the chapter 6 tables. Parse by column position,
   per table (headers move between pages); a parser that disagrees is checked against the raw line
   before the source is doubted.
4. **Names as bacnetio reads them**: compare property names with the BACnet library's identifiers
   (`bacnet-types` `property_id.rs`); note each correction in the code (KBOB writes
   `Protocol revision`, `Max_APDU_Length_Supported`, `Log_DeviceObjectProperty`, `Enable`,
   `Alarm_Value` of multi-state objects).
5. **Only what the source demands**: read the exact sentence (5.3.4 asks for a BACnetEngineeringUnit,
   and `no-units` is one; 5.3.6 names inputs and values, not outputs). Where the source gives an
   example (AKS pattern) it is a rule the user switches on, not one on by default. Where it leaves a
   choice to the owner, it is not checked. Where two places of the source disagree, the binding
   document wins (the recommendation over its checklist), and the code says so.
6. **Tests** for each rule: a device that meets it, one that breaks only it, and the source's edge
   cases. Write down in the concept what was compared and what is not checked.

Answer the maintainer in German, with the chapter for each claim.
