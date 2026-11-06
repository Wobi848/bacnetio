---
name: new-text
description: "Add or change words the bacnetio interface shows, in all five languages (web/lang/en, de, fr, it, es), including server notes, translated errors and object type names. Use for every change that puts new or different text in front of the user, even a single button label or error message, so no language is left behind."
---

# Texts in all languages

The interface has one file per language in `web/lang/`: `en.js`, `de.js`, `fr.js`, `it.js`, `es.js`,
with the same keys (`docs/concept-languages.md`). `tests/web/ui.test.js` fails when a key is missing
in a language or a key used in `web/` does not exist.

## Steps

1. Pick a key in the area's style (`dev.*`, `pts.*`, `rec.*`, `cmd.*` …); reuse an existing key when
   the words mean the same thing.
2. Add it to **every** file under `texts`, at the same place. Placeholders as `{name}`, the same in
   every language.
3. Words from the server:
   - a note (code from `src/diagnosis.rs` or `points.rs`): add under `notes` in every language
     except English, which comes from the server
   - a known error: add the pattern once to `ERROR_PATTERNS` in `web/i18n.js` and its text under
     `errors` in every language except English (English only where it says it better)
4. Terms: follow the glossary in `docs/concept-languages.md` section 4. BACnet words stay as the
   trade uses them (COV, BBMD, Who-Is, object and property names). German in Swiss spelling (ss).
5. French, Italian and Spanish are drafts until someone from the trade reviewed them; they stay out
   of the menu (`LANGUAGES` in `web/i18n.js`) and open with `?lang=fr|it|es`.
6. Run `node --test tests/web/*.test.js` and the layout test in all languages
   (`node tests/ui/run.mjs layout`): French and Spanish are longer and must not cut anything off.

Never build words in code by string pieces that break in other languages; give the whole sentence
a key with placeholders, and plural forms their own keys.
