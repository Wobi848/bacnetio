# Quality plan: from here to field-ready

bacnetio has to reach the same standard as its sister project mbusio (`../mbusio`, next to this
repo). Work through this plan in order, without stopping between steps, until "Done" below holds.
Tick each box when its step is finished and commit this file with it.

Talk to the maintainer in German. Everything in the repo and on GitHub stays in English.

## When to stop and ask

Keep going unless one of these comes up. Then ask, and continue with whatever does not depend on
the answer.

- A commit or merge to `main`. Merging needs the maintainer's OK and green CI.
- A tag, release, npm publication or Windows build. Do not offer them either.
- Making the repo public.
- Removing or weakening a function or control.
- Writing to a real BACnet device, or anything on a real site.
- A product decision the concepts leave open.

## Step 1: Clean start

- [x] Move the unfinished work from `main` to a topic branch (`git switch -c <topic>`) and finish it.

From now on: one short branch per topic, never commit to `main`.

## Step 2: Working rules

- [x] Create `AGENTS.md`, with `../mbusio/AGENTS.md` as the model. Move the current content of
      `CLAUDE.md` into it and leave only `@AGENTS.md` in `CLAUDE.md`. Do not change anything in
      mbusio.

It has to contain these rules:

**Git and GitHub**
- Commits, branches, PRs, issues, tags, release notes, file names and identifiers in English.
- No AI attribution: no `Co-Authored-By`, no "Generated with".
- Stage your own files by name, never `git add -A`. Another AI session may be working in the repo.
- Separate commits per concern; the subject says what changes.

**Releases and publishing**
- Tags, releases, npm and Windows builds only on the maintainer's explicit request.
- Before the repo goes public: no secrets, private addresses or personal details, including in the
  Git history. Android signing keys never enter the repo.

**Before saying "done"**
- Run everything CI runs, plus the browser tests: `cargo fmt --check`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `node --test tests/web/*.test.js`.
- Say what could not be checked (real devices, real networks, other browsers).
- Never say "done" while known points are open. Name them and work through them; do not stop at
  "the next step would be".
- Bigger changes: concept in `docs/` first, then code.
- User-facing changes go in `CHANGELOG.md` under "Unreleased".

**Never lose a function**
- Before replacing or removing something, list every function of the old version and show each one
  in the new version, preferably with a test. Old tests still passing is not enough.

**Operation**
- This is a work tool: self-explanatory, clean, every function reachable.
- Buttons look like buttons and name their task: word before symbol, no borderless text buttons.
  Clarity before density: rather two lines than one word less.
- Never weaken a control unasked (paler, borderless, symbol only).
- The UI model is the mbusio working folder on branch `ui-sidebar`, not its `main` or GitHub. That
  state is not committed or approved yet, so it may still change. Take over structure and patterns,
  and check screens with screenshots.

**Languages**
- New texts need every existing language; a test must fail otherwise.
- A new language enters the menu only after review by someone from the trade.

**Real plants**
- Never write to a real BACnet device unasked, not even to test. Tests run against the simulators
  in `examples/`.
- Every write from the UI stays behind the session unlock, with a clear confirmation and a log entry.

## Step 2b: Findings from the review of e818392

Do these first. The first two touch the safety of real plants.

- [x] The server accepts requests with any `Host` and `Origin`. A write with
      `Host: evil.example:18099` and `Origin: http://evil.example:18099` changed a value on the
      simulator, so a web page could reach a plant through DNS rebinding. Answer 403 unless `Host`
      is the server's own loopback address and port, and `Origin`, when sent, matches it. Add a test.
- [x] Setting a device's clock (`POST /api/devices/{instance}/clock`) writes no log entry. Log it
      like every other write, and add a test.
- [x] Sidebar: "Ready to search" is cut off to "Ready to searc" next to "Change". The browser tests
      must catch this.
- [x] Devices: the chosen device shows "Open data points" twice, at the top right and in the details.
      Keep the next step at the top right unless the second button has a reason to stay.
- [x] Split `src/present.rs` (425 lines) and `src/server/devices.rs` (403), and shorten
      `scan::run_in_range` (88 lines), once other work there is committed.

## Step 3: Browser tests

Today's tests in `tests/web/` do not run in a browser, so clipped texts go unnoticed.

- [x] Add Playwright tests modelled on mbusio (`tests/ui/README.md`, `tests/ui/27-layout.test.mjs`).
- [x] Check every page at phone, medium and wide window width, in light and dark.
- [x] Fail on clipped texts, buttons or status chips, and on horizontal page scrolling.
- [x] Use long names and many entries as test data, including 170+ points.
- [x] Run the layout tests in Chromium, Firefox and WebKit.
- [x] Add them to CI as far as the runners allow, and fix what they find.

## Step 4: Changelog and function list

- [x] Add `CHANGELOG.md` with an "Unreleased" section.
- [x] Extend the existing function list in `docs/concept.md` section 3: each function gets its place
      in the UI and its test. Do not start a second list.
- [x] Add a test for each function that has none. Report anything that does not work.
- [x] Add a test that fails when a text is missing in a language, if there is none yet.

## Step 4b: The same languages as mbusio

bacnetio gets English, German, French, Italian and Spanish. This changes the two-language scope in
`docs/concept.md` section 4, so update it there.

- [x] Write the concept first, modelled on `../mbusio/docs/concept-languages.md`: interface, server
      messages, exports, Android texts, date and number formats, a shared glossary.
- [x] Split `web/i18n.js` into one file per language with the same keys, and fall back to English.
- [x] Translate everything into French, Italian and Spanish. Keep BACnet terms from the standard
      where the trade uses them, such as object and property names.
- [x] Keep French, Italian and Spanish out of the menu until someone from the trade has reviewed
      them. Reviewers open them with `?lang=fr`, `?lang=it`, `?lang=es`, as in mbusio.
- [x] Test that every language has the same keys, and run the layout tests in every language at
      phone, medium and wide width. French and Spanish are longer than English.

## Step 4c: Redesign concept

The maintainer wants a redesign. Bacnetio should feel like one workspace for browsing a plant, not
a row of pages.

- [x] Write `docs/concept-redesign.md` with a clickable mockup, building on
      `docs/concept-explorer.md`. Include the function list and where each function lives on.
- [x] Cover these proposals:
      - One workspace: tree, table and details side by side, replacing the separate device and point
        pages.
      - Site problems: everything overridden, out of service, faulty or offline across all devices.
      - Priority array as the centre of a point: 16 slots, who holds what, what wins now. Warn before
        a write when a higher priority would override it.
      - Leaving check: values you still hold stay visible and warn before closing, also after a
        restart.
      - Search across the whole site, not just one device.
- [x] Stop and ask: the maintainer has to approve the concept before step 5 C is built. Continue
      with what does not depend on it.

## Step 5: The best BACnet browser

- [x] A: Live search (`docs/concept-explorer.md` section 3)
- [x] B: Request planning, adaptive timeouts, background reading, request log
- [x] A2: Read object lists during the search (`docs/concept-explorer.md` section 2.3), not only when
      a device opens. The maintainer's other BACnet browser does this, and today's search still
      stops at device facts. Build it now, it does not wait for the tree: read identifiers only,
      in the background with limited parallel requests, without delaying the first devices. Show
      each device's object count and progress in the device list, and keep the list when the
      device opens. A failed list is a note on that device and never fails the search. Test with
      the simulators, including many devices and one that does not answer.
- [x] C: Explorer tree. It replaces pages, so the "never lose a function" rule applies in full.
- [x] D: Site memory (the last search; several sites in step 5b), watch list, EDE export, alarms

Each step meets its "done when" from that concept. Where this takes a real device, prove what you
can against the simulators and put the rest on the checklist in step 7.

## Step 5b: What is still missing against the K&P browser and YABE

The maintainer asked to build on until everything is done, so these follow the workspace. Decisions
as in `docs/concept-explorer.md` section 2b, "step C" and "step D".

- [x] Device by address also behind a router (network number and MAC); only the field can prove it
- [x] Properties with their data type, updated every 10 s, changes marked
- [x] Write other properties than the present value, behind the unlock and logged: out of service,
      description, name, COV increment, limits, deadband
- [x] The priority that wins now on each point, as a chip, read when asked (*Read priorities*), because it takes one
      request per object
- [x] Values pushed by unconfirmed COV shown as live
- [x] Several sites in the site memory, with export and import as a file

Next, asked for by the maintainer: the same choice for recording, and MS/TP over an RS-485 adapter (only with a real adapter to test on).

Marked "later" there and not part of this plan: comparing devices side
by side, pcap export, own grouping patterns, mass operations, BBMD tables, print layout, client device
ID, backup and restore. Excel files are not built either. Built beyond the list because a browser
needs them on site: device ID, device commands, acknowledging alarms, Who-Has, schedule and calendar
views, writing any plain property.

## Step 6: Design pass

- [x] Compare every page with the mbusio model, with screenshots in light, dark and on a phone.
- [x] Fix everything that looks unfinished or needs an explanation.
- [x] Make sure a first-time user can find the next step on every page without help (concept
      section 3.1).

## Step 7: Hand-over for the field

- [x] Add `docs/field-test.md`: what the maintainer checks on a real site (DDC4000 over VPN, a
      certified controller, MS/TP behind routers, without administrator rights on Windows),
      side by side with YABE, and what result to expect.

## Step 7b: First field test, 2 November 2026

The maintainer tested the Windows build on a K&P DDC4000 #31 (553 objects) in the office. Every
remark goes here, done or not, so that no session loses one. Writing on priorities works on the real
device.

Fixed on branch `field-test-fixes`:

- [x] Toggle buttons unreadable under the mouse.
- [x] "device 31 not in device table" after 10 minutes: the client forgot devices; a task now asks
      them again (checked on the simulator after 11.5 minutes).
- [x] Priorities did not follow a write from the building management at once.
- [x] A slot written over by the plant still said *You*.
- [x] No way to release a priority another system wrote.
- [x] *Overridden* unexplained (it is a local override, not a priority).
- [x] Device refusals as raw BACnet codes (write-access-denied, password-failure).
- [x] Large value with gaps around the decimal comma; *inactive* untranslated in plant problems.
- [x] Write form: value field ran under the priority; "0 %" broke between number and unit.
- [x] Chart axis showed 0,09 at top and bottom; recorded names in capitals (b became B).
- [x] A point renamed to "Test" was not found by the search.
- [x] Device ID change: retry as wildcard 4194303 when the device does not know its reported ID.

Open, in the maintainer's words where it helps. The larger ones are planned in
`concept-field-feedback.md`:

- [x] **Plant groups:** K&P names give one group per point (`001/00/00/P.01/CAI` with only `b`).
      Group a level higher so a plant part holds several points.
- [x] Recording start easy to miss: a note with a *Start* button.
- [x] **Recording:** COV or polling chosen per
      point, COV recording every report with its time; marks for each measurement, gaps in red,
      *last measured* with a short pulse ("dass man sieht holt werte aber der hat sich nicht
      geändert").
- [x] **Technical details:** offer the unlock there; an analog input has no *Change value* button
      to unlock from.
- [x] **Search, filter, sort** to be extended ("allgemein filter sort und so sollten ausgebaut
      werden").
- [x] **Design overhaul** ("das Design sollte überarbeitet werden"): concept in `docs/` first, shown
      to the maintainer before building. Concept: `docs/concept-design.md`.
- [x] **Sites as a menu item** of their own instead of a card in the settings: named by the user,
      not by IP; a page listing them with open, rename, save to file, remove; the open site's name
      at the top of the sidebar.
- [x] **Quick actions in the list:** small icons in each row to mark a point (watch list) and to
      add it to the recording, shown filled when on; the buttons in the details to look better.
- [x] *Held by you* stayed on a slot the plant has written over (and *Release all* would have
      released the plant's value).
- [ ] Read errors on `S_01.01/h` and `S_01.02/h`: waiting for a screenshot of the details.
- [ ] Field visit 3 November: 7 drives on MS/TP with device IDs over 9,000,000 (BACnet allows up
      to 4,194,302). Reachable only behind a BACnet router; changing an ID behind a router and the
      wildcard retry are untested on real devices. Direct MS/TP over RS-485 is not supported.
      The drives all answered as 4194303; giving them new IDs one by one worked, but a search did
      not replace the address the client had kept for 4194303, so the next change went to a drive
      already changed (*unknown-object*) until bacnetio was restarted.
- [x] A search starts with an empty device table; a write goes to the address shown in the
      details (`scan::aim_at`), not to whichever device with that ID the client heard first.
- [ ] **Duplicate IDs throughout:** reading, the point list, watch list and recording still go by
      ID, so with duplicates they show one of the devices only. Address such devices by network
      and MAC everywhere, and let the list open each of them. Concept first. Concept:
      `docs/concept-duplicate-ids.md`, waiting for the maintainer.
- [x] The unlock question stuck out of narrow details (button text did not break).
- [x] **Details wait for the list on MS/TP** ("da muss ich immer warten bis er alle objekte
      gelesen hat"): the client lets one request at a time through a router to one network, and
      the reading of the list takes turns with the details. A point opened by the user should go
      first; measure on the plant with the request log before building. Built 3 November without
      the measurement (`src/precedence.rs`): the reading pauses while a detail request runs; how
      much faster it is on the plant is still to see.
- [x] Device list: network and MAC instead of a cut-off *behind router 10.3…*.
- [x] **Filters in site problems and the watch list** ("in der Merkliste fehlen noch filter sort
      funktionen", "Anlagen Probleme genau so"): the search of the point list, states as views,
      one device. Rows were also out of line.
- [x] **Active alarms crowd the device details** ("das überfüllt mir da die Ansicht"): count,
      the first five, names, states in words.
- [x] **Columns to resize** ("die Spalten … das ich Adresse grösser ziehen könnte"): drag the
      border between two column heads in the device list, the point list, site problems and the
      watch list; width remembered per browser, double-click for the automatic width, like the
      panels.
- [x] **Rename a device** (its object name): a NovoCon kept the name `NovoCon-#4194303` after its
      new ID.
- [x] **Note when a new version is out** ("anzeigen falls eine neue Version released wurde"): asks
      GitHub once a day, a link in the sidebar, a switch in the settings
      (docs/concept-update-check.md).
- [x] Area keys `1` to `6` instead of `0` to `5`, side by side on the keyboard.
- [x] Accent colour to choose ("ich liebe Crimson", "ich mag das Blau nicht"): blue, crimson,
      violet, teal.
- [x] **Panels to resize** ("Panels verschieben"): by default the widths adapt by themselves to
      the window and the content ("möglichst immer optimal"). A user who wants to can drag the
      borders between sidebar, tree, list and details; that width is remembered per browser, and a
      double-click hands the panel back to the automatic width. Part of the design overhaul.
- [x] **Backgrounds to choose** as well, not only the accent ("die BGs und so anpassen"): a few
      tested background tones per theme (e.g. the navy now, neutral grey, warm, black), each checked
      for contrast like the accents. Comes with the design overhaul.
- [x] **Logo:** the maintainer offers `Glossy Blue AI Network Emblem.png` (1254 px, blue to white
      network shape) for the app. Its right half is near white, so on light backgrounds and as a
      16 px browser icon it needs a variant with that half in colour, or a simplified small icon.
      Decide with the design overhaul; the file is not in the repo yet. Built 3 November: the
      emblem with its right half in colour (`assets/logo.png`, the exe from 48 px), a simplified
      cross with four dots for the tab, the sidebar and the exe up to 32 px.
- [x] **Properties readable** ("die Properties könnten wir noch schöner darstellen"): sections,
      names in the interface's language with a switch to the BACnet names, values by meaning (Yes/No,
      flags by name, time stamps, units). Concept: `docs/concept-properties.md`, mockup shown.
- [ ] **Basics first, then extras** (2 November: "Alle Grundfunktionen müssen zuerst perfekt
      funktionieren bevor wir irgendwelchen KBOB-Zeugs rein machen"). The KBOB check and similar
      extras wait until finding, reading, writing, the point list, recording and sites work without
      fault. Then they come as an *extension*, off by default and switched on under *Settings*,
      since not everyone needs them ("braucht ja nicht jeder"). The code keeps each extension in a
      module of its own; a loader for third-party plugins only when a second extension asks for it.
- [x] **Phone test through a VPN app, 4 November:** a DDC4000 found over the VPN by asking each
      address (no broadcast passes). Wishes from it: a VPN chosen before mobile data, the note to
      name the VPN instead of the firewall, the Android back key to step back and to ask before
      leaving, and more room for the list on a phone ("der Oberteil nimmt zu viel Platz ein"): the
      head folds to one line (site, devices, settings) and unfolds adapter, *Change* and the site
      search on a tap; the tabs swipe in one line; watch and record sit beside the value.
- [ ] **Drag to switch many at once**, as in Blender (4 November: "wenn man mit gedrückter
      maustaste darüber fährt das man alle gleich markiert oder auswählt"): press on one switch,
      such as watch or record in a row, and drag over others: they all take the state of the first.
      Concept: `docs/concept-drag-toggle.md`, waiting for the maintainer.
- Decided: the instance of other objects cannot be changed (devices rarely allow it, and
  references break). Revisit only for a concrete case.

## Step 8: Going public

The maintainer wants to publish bacnetio once it works ("irgendwann will ich öffentlich damit
gehen wenn es funktioniert"). Publishing cannot be taken back, so each point is checked first and
the switch itself is the maintainer's.

- [x] `CONTRIBUTING.md`, templates for pull requests and issues, `SECURITY.md`.
- [x] Tree and history searched for real site addresses and personal data (3 November: none
      left; one example address replaced before it was pushed).
- [x] Commits carry the maintainer's no-reply address. The history was folded into one commit
      before going public; the old one is kept by the maintainer outside GitHub.
- [x] The logo emblem: made with ChatGPT, whose terms let the maintainer publish it. As an AI image
      it is likely not protected by copyright, so others may reuse it; accepted for now.
- [x] `AGENTS.md` names things only the maintainer has (the mbusio folder as the model, another AI
      session in the same folder): worded for anyone; the maintainer's own notes moved to
      `CLAUDE.local.md`, which Git ignores.
- [x] On GitHub: `main` protected (pull request and green CI required), private vulnerability
      reporting on, a short description and topics. Public since 3 November 2026.
- [x] CI runs again, free for a public repo. The stacked branches are merged; the last fix left on
      `plant-tools` came over with `going-public`.
- [ ] A first release with the Windows build, on the maintainer's word.

## Done

All boxes are ticked, every check from step 2 is green, and all branches are ready to merge. Then
report to the maintainer in German: what is finished, what is open, what could not be checked.

The field and usability tests and the first release (milestone M3) are the maintainer's call and
are not part of this plan.
