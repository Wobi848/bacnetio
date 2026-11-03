# Concept: design overhaul

The maintainer finds the design not yet good enough (field test, 2 November 2026). This concept
lists the weak spots seen on screenshots of every page, light and dark, at 1366 and 1920 px, and
what to do about each. It is built only after the maintainer has read and approved it, one part at
a time, in the order of the last section. Numbers refer to the marked screenshots shown to the
maintainer.

Rules that stay: buttons are recognisable as buttons and say what they do in a word, clarity before
density, nothing weakened without asking (see `concept.md`). Every function keeps a place.

## 1. The toolbar of the point list takes three rows (1, 2, 3)

**Now:** search and views in one row, five buttons in a second, the type chips in a third, the
removable filters in a fourth. The list starts low, on a laptop a quarter of the height is gone.
The type chips repeat what the tree already shows under *By type*.

**Then:** one row.

- Search (wider, the prefixes `#`, `type:`, `value:` in its tooltip and under it while typing),
  the views (*All · Faults · Overridden* and the rare ones while they have points).
- A button *Types* opens a list of the device's types with counts and a tick box each; several at
  once, as the chips now.
- On the right *Read again*, *Live values* and *More …*. *More …* holds what is used now and then:
  device details, export (CSV, priorities, EDE, state texts), open or close all groups, save the
  reference, read winning priorities.
- The removable filters keep their own row, shown only while something is filtered.

## 2. Every button looks the same (2)

**Now:** almost every button has the accent outline. The one next step and a rarely used export
look equal.

**Then:** three kinds, each clearly a button with border, background and a word:

- *Next step:* filled in the accent colour, one per page (*Find devices*, *Start recording*).
- *Tool:* surface background, grey border, text in the text colour; accent only under the mouse.
- *Toggle:* the segmented rows; the chosen one filled in the accent colour.

## 3. The details cover the list on a laptop (4)

**Now:** below 1400 px the details of a point slide over the list and hide the values.

**Then:** from 1200 px the details are a third column beside the list (about 340 px); the
description column gives way first. Below 1200 px they stay a drawer. With section 6 the user can
widen or narrow the column.

## 4. The sidebar says the network twice (5, 6)

**Now:** the site card shows `127.0.0.0/29`, a line below the adapter repeats it, and the
status bar names the adapter a second time. The search field cuts its own placeholder (*Ganze Anlage durchsuch…*).

**Then:** the site card holds the site's name and network. Below it one line: the adapter as a
menu and *Change*. The repeated line goes. The search field says *Search the site* and shows
its key `/`.

## 5. Problems and watch list are lines without heads (7)

**Now:** one line per point across the whole width, name on the left, value far on the right, no
column heads, no sorting. At 1920 px the eye loses the line.

**Then:** the same table as the point list: heads *Device · Name · Value · State*, sortable,
columns sized to their content; a click opens the point in its device. The watch list keeps
*Remove* per row, as a tool button.

## 6. Panels to resize

As wished on 2 November: the widths adapt by themselves to the window and the content. A user who
wants to can drag the borders between sidebar, tree, list and details; that width is remembered in
the browser, and a double-click hands the panel back to the automatic width. Mouse, and keyboard
on the focused border (arrow keys).

## 7. Recording charts are large and say little (8, 9)

**Now:** each chart is 400 px high even for a flat line; the time axis shows only start and end; a
binary point is drawn on an axis from −1 to 1.

**Then:** charts 220 px high, time marks at round minutes or hours, binary and multi-state points
on their own states (*Off / On*, the state texts) instead of numbers. Single view: two charts side
by side from 1600 px.

## 8. Backgrounds to choose

As wished on 2 November: beside the accent, a background under *Settings → Appearance*.

- Dark: *Navy* (now), *Graphite* (neutral grey), *Black* (for dark rooms and OLED screens).
- Light: *White* (now), *Grey*, *Warm*.

Each is checked for contrast as the accents are (text at least 4.5:1, states and accent at least
3:1 on every surface). The sidebar stays darker than the content in every choice.

## 9. Logo

The maintainer offers *Glossy Blue AI Network Emblem* (blue to white network shape). Its right
half is near white: on light backgrounds and as a 16 px browser icon that half disappears. So two
forms: the full emblem with its right half in a light blue, for the sidebar and the Windows
program; a simplified small icon (the centre cross with four dots) for the browser tab. Both are
shown before they go in.

## 10. Order of building

Built so far: parts 1 to 4 and the backgrounds of part 5; the logo waits for the maintainer's choice. Charts of states are lower than charts of numbers in the comparison,
since two or three states need less room; the single view already had two charts side by side from
720 px. The line below the adapter shows only while the next search would look
elsewhere than the open site. The views of the point list wrap onto a second line in a narrow list
instead of being cut off; the toolbar stays at two rows on a laptop, because the views do not fit
beside the search there. Between 440 and 560 px of list width the state stays in its row and only
ID and description give way.


1. Buttons (2) and the toolbar (1): the biggest gain for the daily list.
2. Details as a column (3) and resizable panels (6), since they touch the same layout.
3. Sidebar (4), problems and watch list (5).
4. Recording charts (7).
5. Backgrounds (8) and logo (9).

Each part on its own branch, with tests and screenshots light and dark at 390, 1366 and 1920 px,
and a Windows build for the maintainer to try at the end.
