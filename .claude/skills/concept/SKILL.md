---
name: concept
description: "Write or update a concept in bacnetio's docs/ before a bigger change, with the function list and open questions, then stop for the maintainer's approval. Use whenever a change is more than a fix: a new page, area or panel, moved or removed controls, anything new that writes to devices, a new language, file format or plugin API change, or when the maintainer asks to plan, propose, redesign or \"überleg dir was\"."
---

# Concepts in bacnetio

AGENTS.md: bigger changes get a concept in `docs/` first, then code. Models: `docs/concept-redesign.md`
(with a clickable mockup), `docs/concept-languages.md`, `docs/concept-plugins.md`, and mbusio's
`docs/concept-*.md`.

## When

- A new page, area or panel, or a control moves or goes away
- Anything new that writes to a device, or a new kind of command
- A new language, export format, file the user keeps, or plugin API change
- The maintainer asks for a plan, a redesign or "überleg dir was"

A bug fix or a small addition inside an existing concept needs none; update that concept's state.

## File and structure

`docs/concept-<topic>.md`, in English, short sentences:

1. **Title and status line**: proposal / approved on <date> / built on <date>, with the branch.
2. **Why**: what the maintainer said or saw (quote briefly in German if it helps), what goes wrong
   today, measured or seen, not assumed.
3. **Proposal**: what the user sees and does, step by step. An ASCII sketch for layouts; a
   clickable mockup in `docs/mockup/<topic>.html` (static data, nothing talks to a device) when
   operation changes.
4. **Writing to plants** (if any): unlock, confirmation in the page, log entry, what the leaving
   check shows, what is refused at the server.
5. **Functions and where they live on**: every function the change touches, its old place, its new
   place, its test. Moves update the list in `docs/concept.md` section 3.3; never a second list.
6. **Compatibility**: stored data, files and API fields keep loading; what goes into `CHANGELOG.md`.
7. **Order of building**: small steps, each leaving everything working.
8. **Open for the decision**: the questions only the maintainer can answer, each with a
   recommendation.

## Then

- Stop and ask the maintainer (in German) to approve, naming the open questions. Build only what
  does not depend on the answer.
- After building: set the status line, tick what is built, keep the function list true.
