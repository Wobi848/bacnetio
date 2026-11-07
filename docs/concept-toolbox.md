# Concept: five functions from the K&P comparison

Status: approved on 7 November 2026 with the recommendation to all four questions in section 8
(branch `concept-toolbox`). Built so far: part 1, the print view, and part 4, the own grouping pattern.

## Why

These are the last rows of [concept-explorer.md](concept-explorer.md) section 2b still marked
*later*. The maintainer asked for all of them: "ja bau das mal alles ein". Each one is a part of its
own and a pull request of its own, built in the order of section 7:

1. **Print view** of a device's points, which the browser saves as PDF.
2. **Compare two devices** side by side.
3. **Capture** bacnetio's own traffic and save it for Wireshark.
4. **Own grouping pattern** per site.
5. **Change many points at once**: one property, one value, many objects.

## 1. Print view

The technician needs a list for a handover or a report. Today there is CSV for Excel, but no page to
hand over.

- *More → Print* in the point list opens the device as a page made to print. bacnetio's sidebar,
  tree, toolbar and details do not show on it.
- The head shows the site's name, the device's name, ID, address, vendor and model, when the values
  were read, and bacnetio's version.
- The points are in the groups of the list, with these columns: type and number, name,
  description, value with its unit, state, and the priority now when known. Faults and overridden
  values are marked by a word, not by colour alone, so a black and white print shows them too.
- The filter, view and object types of the list stay in force, so the technician can print only the
  faults, for example. The head says what is shown: "Faults only, 4 of 181 points".
- *Print …* opens the browser's print dialog. There the technician can also choose
  *Save as PDF*. *Back to the list* returns. Ctrl+P in the list prints this view too.
- A4 portrait. A group does not start at the foot of a page, and the column heads repeat on every
  page. The head also says when the page was made and with which bacnetio.

Nothing is sent to a device. The page uses what the list already has.

## 2. Compare two devices

Two air handling units of the same kind should be set up alike, and two controllers of the same
type should have the same points. Today the technician opens one, then the other.

- *More → Compare with …* in the point list asks for a second device of the site. It offers
  devices of the same vendor and model first.
- The list then shows the points of both devices in one table, matched by name. The shared start
  of the names (as in the grouping) and the device's own prefix are left out. Columns: name; type
  and number, value and state on the left; the same on the right.
- Views: *All*, *Differences* (another value, state, unit or type), *Only left*, *Only right*.
  A difference is marked in the cell, with a word in the tooltip.
- The values are the ones read. *Read both again* reads both devices. Live values work as in the
  list, for the points on screen.
- A click on a side opens that point in the details, as from the list.
- *Close comparison* returns to the one device.

Nothing is written. The comparison against a saved state of the same device ("since the
reference") stays as it is.

## 3. Capture for Wireshark

On a technician's laptop without administrator rights, Wireshark cannot capture: its driver needs
an installation. bacnetio sees its own traffic anyway.

- *Settings*, card *Capture*: *Start capture* and *Stop capture*. While a capture runs, the status
  bar shows *Capturing · 1 234 packets*, and a click on it leads to the card. bacnetio has no
  *Advanced* area, and a capture is set up once per visit, like the other cards there.
- *Save as pcap* downloads what was captured. Wireshark opens the file and decodes BACnet as if it
  had captured it itself.
- What is in it: every BACnet packet bacnetio sends and receives, with its time, on IP and on
  MS/TP. Other programs' traffic is not in it. A broadcast that reaches bacnetio is in it, since
  bacnetio receives it.
- On IP the file holds IPv4/UDP packets with their BVLC header. The headers below the BVLC
  (Ethernet, IP and UDP) are built from the addresses, since bacnetio sees no frames. The
  file says so in its comment. On MS/TP the frames are in the MS/TP format Wireshark knows
  (link type 165), without the token frames, since the library hands over only the frames with
  data.
- Not more than 100 000 packets: past that the oldest go, and the page says so. The capture lives
  in memory and is gone after a restart. *Clear* empties it.
- Capturing is off until started. It costs nothing while off.

How: a wrapper around the transport records each packet sent and received before passing it on.
It holds the time, the direction, the two addresses and the bytes. The `Client` type becomes a
client over this wrapper; nothing else in bacnetio sees the change.

## 4. Own grouping pattern per site

The automatic grouping finds plant parts in path names (`/21/000/09/S337.01/5101`). Some sites use
other keys, such as `ZUL_VEN_01_TT01` or `B12.AHU1.SupplyTemp`, which it groups by type only.

- *More → Grouping …* in the point list: *Automatic* (as today) or *Own pattern* with a field for a
  regular expression. Its groups in brackets form the plant part, and the rest of the name is the
  point's name. Example: `^(\w+_\w+_\d+)_` gives *ZUL_VEN_01* with the point *TT01*.
- Under the field a preview takes the first points of the open device and shows "*ZUL_VEN_01* ·
  TT01" for each. It says how many points fit and how many fall under *Other*.
- An invalid pattern says why in words and is not saved. Spaces count in a pattern; one of
  nothing but spaces is none.
- The pattern belongs to the site and holds for all of its devices: in the list, the tree, the
  print view, the comparison and the site search. It is saved with the site and comes back when
  the site is opened.
- A site without its own pattern groups as today.

Nothing is sent to a device.

## 5. Change many points at once

Some changes have to be made at many points at once: out of service on the inputs of a module for a
test, a new COV increment, or the value of all the setpoints of one group. Doing them one by one
takes long and is easy to get wrong.

- In the point list *Select* switches the rows to checkboxes. *All in view* selects what the filter
  and the view show, and a group's head selects that group. The toolbar shows "12 selected" and
  *Change selected …*.
- *Change selected …* asks for the property, from the ones all selected points have and that can
  be written in the details: present value with priority, release a priority, out of service,
  COV increment, description, and alarm limits.
- Then a table shows each point with its value now and its new value. A point where nothing would
  change is marked so. A point that cannot take the value (a state number beyond its texts, a text
  where a number is needed) is marked and left out.
- *Change 12 points* asks once more in words: "Write out of service = Yes to 12 points of
  AHU-01?". Then bacnetio writes one point after the other and shows each one's result in the
  table: done, refused by the device (with the reason in words) or not reached.
- *Stop* ends it after the point being written. What is done stays done; the table says which
  points were written.

### Writing to plants

- Behind the session unlock, as every write. Without it, *Change selected …* first asks to allow
  changes.
- One device at a time and not more than 200 points per change. The server refuses more.
- The server checks each value as for a single write (the same check). A point that fails the
  check is left out with the reason. The rest are written.
- Every point gets its own log entry, refused or not, as a single write gets. A present value
  written with a priority counts as held by you and shows in the leaving check.
- The confirmation names the device, the property, the value, the priority and how many points.

## 6. Functions and where they live

Nothing goes away or moves. New rows for `docs/concept.md` section 3.3, each with its test once
built:

| Function | Where | Test |
| --- | --- | --- |
| Print view of a device's points, filters kept | point list, *More → Print* | `ui/40-print` |
| Compare two devices by point name | point list, *More → Compare with …* | `ui/41-compare`, `tests/web` |
| Capture traffic and save as pcap | *Settings*, card *Capture*; status bar while running | `tests/capture.rs`, `ui/42-capture` |
| Own grouping pattern per site | point list, *More → Grouping …* | `tests/web/grouping.test.js`, `site.rs`, `ui/43-grouping` |
| Change one property of many points | point list, *Select*, *Change selected …* | `tests/api.rs`, simulator test, `ui/44-change-many` |

## Compatibility

- The site file gains a field `grouping` (the pattern). Older files have none and group as today.
  Older bacnetio versions ignore the field.
- New API routes only; none changes. They go in `docs/api.md`.
- Each part goes in `CHANGELOG.md` under *Added*.

## 7. Order of building

Each step is a pull request and leaves everything working:

1. Print view (only the page).
2. Own grouping pattern (site file, a route, the grouping).
3. Compare two devices (the page, using the grouping).
4. Capture (the transport wrapper, routes, the page).
5. Change many points (the server route with its checks and log, then the page).

## 8. Decisions

1. **Compare:** two devices matched by point name, as above. The alternative is to pin single
   points side by side, but the watch list already shows points of several devices. *Recommended:
   devices.*
2. **Change many, which properties:** present value with priority, release, out of service, COV
   increment, description and alarm limits; one device at a time; at most 200 points.
   *Recommended: as listed.*
3. **Capture:** started and stopped by hand, as above, or always on, keeping the last packets.
   *Recommended: by hand*, so that nothing is recorded unasked and memory stays free.
4. **Grouping pattern:** per site, saved in the site file. The alternative is one pattern for
   everything in the browser. *Recommended: per site*, since keys differ from site to site.
