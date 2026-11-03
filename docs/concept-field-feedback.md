# Concept: after the first field test

The maintainer tested bacnetio on a K&P DDC4000 with 553 objects on 2 November 2026 (findings in
`quality-plan.md`, step 7b). The small fixes are built. This concept covers the larger wishes. It is
built only after the maintainer has read and approved it, one part at a time, in the order of
section 7.

## 1. Plant groups that hold several points

**Now:** the group is the object name without its last segment. K&P names such as
`/31/001/00/00/P.01/CAI/b` give one group per point (`001/00/00/P.01/CAI` holding only `b`). Half the
list is group heads, and the plant tree has hundreds of entries with a 1.

**Then:** the depth of the groups adapts to the names. bacnetio tries the depths from deep to
shallow and takes the deepest one at which a group holds at least three points on average. For the
DDC4000 that is probably `001/00/00`; the rest of the path stays in the row's name
(`P.01/CAI/b`), so nothing is lost. Devices whose names already group well keep their groups.

The tree under *Plant* follows the same groups and can open one level further, so a technician
who wants `P.01/CAI` alone still gets it. Tested with the K&P names from the field test and with
the simulator's names.

## 2. Recording: COV or polling per point, and seeing that it measures

**Now:** every point is sampled at the chosen interval, from a COV report where the device sends
them. A change between two samples is lost, and a flat line does not show whether bacnetio still
measures.

**Then:**

- Per point a choice *COV* or *Poll*, shown in the figures table and in the CSV.
  - *COV:* subscribes and records every report with its own time. A flat line between reports is
    drawn as such. If the device refuses COV, the point says so and falls back to polling.
  - *Poll:* reads at the chosen interval, as now.
- Each measurement gets a mark: small dots on the line where there are few, a thin strip of ticks
  under the chart where there are many. A measurement without answer is a red tick, so gaps show.
- Each card says *Last measured 12:32:38*, which flashes briefly at every new measurement.

## 3. Sites as a menu item of their own

**Now:** the kept sites are a card in the settings, named by their address (`192.168.10.32`).

**Then:**

- A menu item *Sites* at the top of the sidebar, above the workspace.
- Each site has a name the technician gives (*Sample School*); the network stays below it
  in small print. A site found by a search is named by its network until renamed.
- The page lists the sites: name, network, devices, last opened. Per site: open, rename, save to a
  file, remove (with the question as now). At the top: *Search a new site* and *Load a site from a
  file*.
- The open site's name stands at the top of the sidebar; a click leads to the list.
- The settings keep language, theme and keys.

Every function of today's card lives on: open, remove, save to file, load from file move to the
page unchanged.

## 4. Quick actions in the list

**Now:** marking a point (watch list) or recording it takes opening its details and a button there.

**Then:**

- Two small icons at the end of each row: an eye for *Watch* and a curve for *Record*. Grey at
  rest, clearer under the mouse, filled and coloured when the point is watched or recorded, so the
  list shows what is already in. A click toggles without opening the details; a short note
  confirms it. Each icon has a tooltip with its word.
- In the details the three buttons below each other become one row of toggles: *Watch · Record ·
  Technical details*, each showing whether it is on.

This is the one place where icons stand for words: marking many rows quickly needs them, and the
words stay in the details and the tooltips.

## 5. Search, filter, sort

**Now:** one search field (name, description, instance), one type menu, the three view toggles
(all, fault, overridden), sorting by clicking a column head.

**Then:**

- The search field understands a few prefixes, shown as a hint under it: `#2796214` for an exact
  instance, `type:AO`, `value:>50`. Plain words search name and description as now.
- The type menu becomes a row of chips with counts (AI 65 · AO 22 · …), several at once.
- The view toggles gain *Alarm*, *Out of service* and *Held by you*.
- What is filtered stays visible as removable chips above the list, with *Clear all*.
- Sorting: a second click on a column sorts descending, a third goes back to the device's order
  (as now); the sorted column is marked more clearly.

## 6. Design overhaul

The maintainer finds the design not yet good enough. The parts above change the list, the details,
the sidebar and the recording. Before building, screenshots of each page in light and dark, on a
laptop (1366 px) and at 1920 px, with the weak spots marked, are shown to the maintainer. Points to
look at: the toolbar of the list takes three rows; buttons look all alike; the details are a long
column.

## 7. Order of building

1. Plant groups (section 1): the biggest gain for the list, small change.
2. Quick actions and the details buttons (section 4).
3. Recording (section 2).
4. Sites (section 3).
5. Search, filter, sort (section 5).
6. Design overhaul (section 6), with screenshots first.

Each part on its own branch, with tests, and a Windows build for the maintainer to try.
