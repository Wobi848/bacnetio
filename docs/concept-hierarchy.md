# Concept: a clear order on every page

Status: approved on 6 November 2026, with yes to the three questions at the end. Built one page per
branch, starting with the point list.

## Why

The maintainer: "Today everything looks the same and you do not know where to look." bacnetio is a
work tool; on every page the eye has to find at once what to do next and what the data say.

The rule comes from the maintainer's design system (*Hierarchy*, *PageHead*, *ListRow*): every page
has three levels, and only one may shout.

1. **The next step:** one filled accent button in the page head, beside one sentence that says what
   to do here. No next step, no filled button.
2. **The data:** the largest and boldest thing on the page. Names in weight 600, descriptions in
   `muted`, values in `mono` right-aligned with their unit, the chosen row `accent-soft`.
3. **The tools:** quiet (`.quiet`), one toolbar row, the rare ones behind *More …*.

State chips only where a state is not normal. No function goes away; no control becomes paler,
borderless or a bare symbol (AGENTS.md). Where the design system asks a control to become quieter,
this concept names it, so the maintainer approves each one.

What `concept-design.md` already built stays: the three kinds of button, the one toolbar row with
*More …*, the details as a third column from 1200 px, the sidebar with one adapter line, the tables
of problems and watch list, charts by state, the backgrounds.

## How it was looked at

Screenshots of every page (devices, data points, site problems, watch list, recording, held by
you, settings), light and dark, at 390, 1366 and 1920 px, in German, 42 in all: four simulators
(`simulated_plant`, `simulated_controller`, `simulated_commands`), one point on the
watch list, one in the recording, one value held. The state: `main` with the open pull requests
of the search beside the release (#19), the plugin events (#21) and the priority names (#22). The
pictures were shown to the maintainer; they are not in the repo.

## Across all pages

**A. Accent fills compete with the next step.** Besides the one next step, these are filled in the
accent colour today: the chosen segment of every segmented row (*All 22*, *By type*, *10 s*,
*All* of the time range, *Compare*, the chosen language, theme, accent and backgrounds, *On*), the
current area in the sidebar, *Look now* in the settings, and on the recording page a second *Start
recording* in the note. On the point list three filled blocks sit in one view (next step, *By type*,
*All 22*); on the recording page five.

Then: the next step is the only filled button. A chosen segment stays clearly chosen: `accent-soft`
ground, `accent` border and label in weight 600, the others as now. This differs from the design
system's *SegmentedControl* (chosen = filled), so it is question 1 below. The sidebar keeps its
current item as it is (it is navigation, not a step on the page).

**B. The page head says facts instead of what to do.** Points: "rusty-bacnet · #1001 ·
127.0.0.2:47809"; recording: "Stopped · 0 values". Then: one sentence that says what to do, with the
count in it ("22 points, 1 held by you. Choose one to see its details."). The device facts stay in
the details, where they are already.

**C. A second button in the page head.** Since #19 *Search again* stands as a tool button beside
*Release the held value*, as the maintainer wished on 5 November. The design system allows no other
button in the page head. Question 2.

**D. Chips for normal states.** The change log shows a green *done* on every row. Then: no chip when
the write went through; *refused* and *failed* keep theirs. The point list, problems and watch list
already show chips only for states that are not normal.

**E. Values.** In *Held by you* the value stands in the middle of the row without its unit
("42,5"). Then: `mono`, right-aligned, with the unit, as in the point list.

**F. Group heads as loud as the data.** *Inputs*, *Outputs* … are set like point names (600, same
size). Then: group heads in `table-head` style (11.5 px capitals, `muted`) on `sunken`, with their
count and chips; the point names are then the boldest thing in the list.

**G. Phone (390 px).** On the point list the page head takes two lines (two buttons), the tools
five rows (back, search, views on two lines, *Object types*, *Read again · Live values · More …*):
the first point starts at half the screen. Then: the next step alone in the head as the design
system says (question 2), back and search in one row. The views wrap onto a second line as the
design system's *SegmentedControl* asks, rather than scrolling sideways and hiding a view; the
tools take two rows. Built so: the first point starts at about 45 % of the screen instead of half.

## Page by page

### 1. Data points (built first)

| Level | Today | Then |
|---|---|---|
| Next step | *Release the held value* (filled) and *Search again* (tool); else *Search again* filled | unchanged, but alone filled (A) |
| Sentence | device vendor, ID, address | "22 points, 1 held by you. Choose one to see its details." (B) |
| Data | names 600 like group heads; two state chips and the row icons wrap a row onto two lines (*Heating valve* at 1920 px: *Held by you*, icons below) | group heads quiet (F); chips and the two row icons in one line, the chip column as wide as its longest chip, rows keep one height |
| Tools | at 1366 px two rows: search and views, then *Object types* alone on the left and *Read again · Live values · More …* on the right; *By type · Plant* and *All 22* filled | views not filled (A); *Object types* moves to the right group: *Object types · Read again · Live values · More …*, one quiet row |
| Details | device facts until a point is chosen | unchanged |

Nothing moves out of the page; *Object types* changes its place in the toolbar only.

Built on 6 November 2026 (branch `hierarchy-points`). Rows of `concept.md` 3.3 touched: "Search points; filter …", "Live
values in the list", "Read the device again" (place unchanged), "Open or close all groups".

### 2. Devices

| Level | Today | Then |
|---|---|---|
| Next step | *Search again* filled | unchanged |
| Sentence | "4 devices in 127.0.0.0/29. Choose one to see its data points." | unchanged, it already says what to do |
| Data | names cut off at 1366 px (*DDC4000_#…*, *Commands-…*) while vendor takes a wide column; the tree repeats the names in the same weight | the name column takes the room first, vendor and object count give way; the tree's names in weight 500, the chosen one 600 on `accent-soft` |
| Notes | two orange strips above the list, also the one that says bacnetio worked around the problem | a note that bacnetio solved by itself (no answer to broadcast, devices asked directly) as an info strip (`.info`), orange only where the user must act |
| Tools | *Device by IP …*, *Export …* | unchanged (quiet) |

Built on 6 November 2026 (branch `hierarchy-devices`): below 760 px of list width the vendor column
goes and stays in the device's details, so the name is shown whole beside ID, address and objects.

### Sidebar (asked for by the maintainer after the devices)

On wide screens the area you are on was filled in the accent colour: a second filled block beside
the page's next step, on every page. Then, as the design system's *SidebarNav*: `accent-soft`
ground, a 3 px bar in `brand` on the left, the icon in `accent`, the label in `text` 650. The same
for *Settings* when it is open. The count of site problems carried a class with no style and showed
in neutral blue; it takes the warning colour as the count of held values. On a phone the row of
areas keeps its underline. Nothing moves.

Built on 6 November 2026 (branch `hierarchy-sidebar`).

The head of the sidebar then follows the design system's *SidebarHead*, given by the maintainer as
the goal: a group *Site* (its name in 15 px bold, never the network alone: while the only name is the
network, a tool button *Give the site a name* that opens the site's name field, ready to type (the
maintainer, 6 November: a placeholder says nothing); the name of a site file otherwise; the network once below it; the state with
dot and word, *3 devices found*; *Switch site*, which was the card leading to the sites) and a
group *Searching on* (the adapter; *Search settings*, which was *Change*; the network the next
search looks at, only when it is not the open site's), then the search below a hairline. The
status bar drops the adapter and address and keeps the port. Below 1000 px the folded head names
the site the same way. Branch `sidebar-head`.

Then, on the maintainer's renewed *SidebarHead* (6 November: the three full-width buttons looked
thrown together): one card (`bg` ground, `line` frame, rounded) with two parts built the same way,
each four lines: label, value in 15 px bold, detail in `muted` (network or address in `mono`, for
the site the state), one tool button of natural width on the left. *Site*: the name, or *Give the
site a name* in its place. *Searching on*: the adapter's name and its address; the adapter is
chosen in *Search settings* now, where the other search settings are (an empty device list still
offers it directly). The dark sidebar gets a card ground of its own in every background. The
search stays below the card, on its own.

Last, on the *SidebarHead* as it stands now (6 November): one card with one part. *Site* as a
select field (the name in 15 px bold, or the network while the site has none; it opens the known
sites and *All sites …*, where a site without a name has its name field ready), below it the
network once and the state, beside them *Search settings* as a framed sliders button (its word
joins it from 280 px of card width). The adapter is setup and sits only in *Search settings*.

With it, after the design system's *Tree*, *Menu* and *Dialog* (6 November): devices in the tree in
600 and the chosen node with the bar in `brand`; the list of sites opens under its field, the
chosen site light and bold, *All sites …* below a hairline, the arrow keys move, Escape closes it
and gives the focus back; *Search settings* is a dialog on a dimmed page with its sentence ("Where
bacnetio looks for devices. Changes apply to the next search."), the fields as a list of facts,
*Cancel* and the one filled button; the focus goes in, Tab stays inside, Escape or *Cancel* gives
it back. Nothing moves: every field of the search settings stays (`ui/20-site`, `ui/33-adapters`,
`ui/04-reload`).

### 3. Site problems

The page closest to the rule: device in `muted`, name 600, value right-aligned, a chip only for what
is wrong, *Read all devices* as the one next step. Then: the chosen view not filled (A); the
sentence ends with what to do: "… Choose one to open it in its device." Nothing moves.

### 4. Watch list

No next step, so no filled button: right. The name is cut off (*…Bueroflaech…*) while the empty
state column and the space before *Remove* take the width. Then: the name takes the free width, the
state column only as wide as a chip, the view not filled (A). *Remove* stays a tool button on every
row. Nothing moves.

Site problems and watch list built on 6 November 2026 (branch `hierarchy-problems`): the
sentence ends with what a click does; the name column takes 3 parts of the free width, the state
column 200 px (problems) and 150 px (watch list), as wide as a state chip.

### 5. Recording

| Level | Today | Then |
|---|---|---|
| Next step | *Start recording* filled in the head and again in the note below | in the head only; the note keeps its sentence ("The recording is not running: start it, then it measures every 10 s.") |
| Sentence | "Stopped · 0 values" | "1 point, stopped. Start the recording to measure every 10 s." (B) |
| Data | the series name small above an empty chart; in the figures table the full path wraps over three lines, *Poll · COV* filled on the row | before the first value the chart card says so in one line; the name in one line (device and the last part of the path, the full path as tooltip), per-row *Poll · COV* not filled (A) |
| Tools | four segmented rows filled (interval, range, compare/single), *Clear values*, *CSV*; in German the label *Alle* (every) stands beside the range's *Alles* (all) and reads like one of its choices | not filled (A); the German label *Intervall*, English keeps *Every*; *Clear values* and *CSV* quiet, as now |

Nothing moves out. Row of 3.3: "Recording: add, interval, start, stop, remove, clear, CSV, backup"
(*Start* stays reachable in the head; the copy in the note goes).

### 6. Held by you

| Level | Today | Then |
|---|---|---|
| Next step | *Release all* filled | unchanged |
| Data | the row begins with "Priority 8" in `mono`; name and "held since 11:18" run together (*Heating valveheld since*, a missing space); value in the middle without unit | columns *Device · Point*, *Priority*, *Value* (right, with unit), *Since*, *Release*; the point name 600 first |
| Log | a green *done* chip on every row | chip only for refused or failed writes (D) |
| Tools | *Leave the site* alone in the toolbar | unchanged |

The missing space is a bug and gets fixed with this page. Nothing moves.

Built on 6 November 2026 after the design system's *PageHeld* (branch `hierarchy-held`): the head
says how many values are held and what to do, the next step names how many it releases (*Release
the held value*, *Release 3 held values*); held rows: the point's name 600, below it device, object
and *held since*, the value with its unit, the priority as a `warn` chip, *Release*; the log newest
first with a chip only for a refused write and the device's reason beside it. *Export log (CSV)*
is new (`GET /api/log.csv`). Functions of the page, each with its test:

| Function | Where | Test |
|---|---|---|
| See what is held, also after a restart | *Held now* | `ui/03-write`, `storage.rs` |
| Release one held value, only while the slot is still ours | *Release* per row | `ui/35-release-check` |
| Release all | the page's next step | `ui/03-write` |
| Every write, release, clock and command, newest first | *Written* | `ui/03-write`, `ui/09-device`, `ui/22-commands` |
| A refused write with the reason | *Written*, chip *Refused* | `ui/21-tools` |
| The log as CSV | *Export log (CSV)* | `ui/03-write`, `export.rs` |
| Leave the site, asked first, releasing what is held | *Leave the site* | `ui/15-leaving`, `ui/20-site` |

### 7. Settings

No next step, so no filled button. Today *Look now* (search for a new version) is filled, and every
chosen choice (language, theme, accent, backgrounds, *On*) is filled: seven filled blocks. Then:
*Look now* a tool button (it is not the next step of the page; the design system makes it quiet,
question 3), the choices as in A. The version card shows a technical second line under "No answer
from GitHub" (*no connection to GitHub: failed to lookup address …*): it goes into the tooltip of
the sentence, readable on hover, so the plain sentence leads.

## Functions

No function goes away. What moves or changes its look, for `concept.md` 3.3:

- *Object types*: within the toolbar of the point list to the right group.
- *Start recording* in the recording's note: the copy goes, the button in the head stays.
- *Look now*: from filled to tool button, same place.
- Chosen segments everywhere: from filled to `accent-soft` with `accent` border (question 1).

Each page's pull request lists the functions of that page and the test that shows each one.

## Order of building

One page per branch, each with its tests (the layout test in every language and width, the page's
own browser test), screenshots light and dark at 390, 1366 and 1920 px, `CHANGELOG.md` and the docs.
After each page the maintainer looks at the screenshots before the next one starts; the maintainer
merges.

1. Data points (with A for the segmented control, as it is shared, and F)
2. Devices
3. Site problems and watch list
4. Held by you
5. Recording
6. Settings

## Decided

Answered by the maintainer on 6 November 2026: yes to 1, 2 and 3 as recommended.


1. **Chosen segments not filled?** Recommended: yes, `accent-soft` ground, `accent` border, label in
   600. Otherwise the point list keeps three filled blocks and the recording five, and the next step
   does not stand out. It changes *SegmentedControl* in the design system; the design system would
   be updated with it.
2. **Search again beside the next step in the page head?** Recommended: keep it as a tool button on
   wide screens (you asked for it on 5 November, and searching is used often there), and on a phone
   show only the next step, *Search again* moving to the toolbar row. The design system's PageHead
   would then say "at most one tool button beside the next step".
3. ***Look now* as a tool button?** Recommended: yes; it is not the next step of the settings, and
   it keeps border, background and its words.
