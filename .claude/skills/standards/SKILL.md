---
name: standards
description: "Take rules from a standard, recommendation or checklist (ASHRAE 135, a labelling directive, an owner's guideline) into bacnetio or a plugin so that they are exactly right: transcribed by machine, checked against every other place the source states them, each with its chapter. Use whenever a check, a table of allowed values or a rule comes from such a document, when the maintainer says a rule must be correct (\"muss 100% stimmen\"), or asks where a rule comes from."
---

# Rules from standards, exactly

A check that reports a fault the source does not demand costs the tool its trust. Every rule must
be traceable to a sentence or table of the source, and nothing stricter than the source.

## Sources

- Sources are kept outside Git (`reference/`, see `.git/info/exclude`); where one is public, note
  where to download it.
- Search them before asking the maintainer anything a source answers.

## Steps

1. **Read the source, do not recall it.** `pdftotext -layout` for PDFs, the XLSX with Python's
   zipfile (no extra libraries). Symbols: ■ (U+25A0) or U+F06E mandatory, U+F06F conditional.
2. **Transcribe by machine** into a table in the code, with the source's numbers (`AI-06`, table,
   chapter) beside each entry; never type a table by hand.
3. **Compare with every other place** the same source states it, by machine: a checklist against
   the appendix it summarises, a table against the chapter that states it. Parse by column position,
   per table (headers move between pages); a parser that disagrees is checked against the raw line
   before the source is doubted.
4. **Names as bacnetio reads them**: compare property names with the BACnet library's identifiers
   (`bacnet-types` `property_id.rs`); note each correction in the code (sources write
   `Protocol revision` or `Max_APDU_Length_Supported` where BACnet has other names).
5. **Only what the source demands**: read the exact sentence (a rule that asks for an engineering
   unit is met by `no-units`, which is one; a rule that names inputs does not cover outputs). Where
   the source gives an example (a naming pattern) it is a rule the user switches on, not one on by default. Where it leaves a
   choice to the owner, it is not checked. Where two places of the source disagree, the binding
   document wins (the recommendation over its checklist), and the code says so.
6. **Tests** for each rule: a device that meets it, one that breaks only it, and the source's edge
   cases. Write down in the concept what was compared and what is not checked.

Answer the maintainer in German, with the chapter for each claim.
