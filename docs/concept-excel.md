# Concept: Excel files beside the CSV exports

Status: approved on 7 November 2026 with the recommendation to all three questions (branch
`concept-excel`).

## Why

The last open item of `docs/concept.md` (*Later*: "Excel export") and of the quality plan. Today
every list leaves bacnetio as CSV. A technician who opens a CSV in Excel gets:

- a semicolon as separator (as in mbusio), which Excel splits on a Swiss or German system but not
  on one set to an English region, where each line opens as one column;
- numbers with a decimal point, read as text or as dates on a system that writes a comma;
- no column widths, no header that stays in view, no filter: each time set up by hand.

The CSV files stay as they are: scripts and macros read them by their English column heads
(`docs/concept-languages.md`).

## Proposal

An *Excel* entry beside each CSV entry, with the same rows:

| Export | Where | Sheets |
| --- | --- | --- |
| Device list | table of the site, export menu | *Devices* |
| A device's points | table of a device, *More* | one sheet named after the device; *Excel with priorities* beside *CSV with priorities*, with the same two columns |
| The whole site | table of the site, export menu: *Excel (all devices)* | *Devices*, then one sheet per device read (named after the device, cut to Excel's 31 characters, made unique) |
| Held by you / log | *Held by you*, beside *CSV* | *Log* |
| Recording | *Recording*, beside *CSV* | *Values*: one row per sampling round, one column per point |

Every sheet:

- the header row in bold, frozen, with Excel's filter on;
- column widths from the longest entry, at most 60 characters;
- numbers as numbers (the value in one column, the unit in its own), so Excel can sum and chart
  them in any region; times as Excel times in local time;
- a first line above the header with the site, the device, when it was read and bacnetio's
  version, as in the print view.

Column heads in the interface's language, since the file is for people; the CSV stays in English
for scripts. The whole-site file reads the devices not read yet first, as the EDE file of all
devices does.

Nothing is written to a device.

## How

`.xlsx` is a zip of XML files. Two ways:

1. **`rust_xlsxwriter`** (MIT/Apache, pure Rust, no system library): writes cells, styles, widths,
   frozen panes and filters, and is tested against Excel's own files. One new dependency.
2. **An own minimal writer**: the zip stored without compression, five XML parts, CRC-32 by hand;
   some 300 lines and every Excel quirk found the hard way.

Recommended: 1. It saves real effort and Excel compatibility is the point of the feature.

As built: each list of `src/export.rs` is built once as a `Table`, which gives the CSV and, in
`src/workbook.rs`, the Excel sheet; so each column exists once. Routes as the CSV ones with `.xlsx`
(`src/server/workbooks.rs`): `/api/devices.xlsx`, `/api/devices/{device}/points.xlsx`,
`/api/devices/{device}/points-priorities.xlsx`, `/api/site.xlsx`, `/api/log.xlsx`,
`/api/recording.xlsx`. The page sends the column heads and sheet names by their English names
(`?heads=` as JSON) and the title line (`?title=`), so all words stay in `web/lang` with the other
texts; a test checks that every head of `src/export.rs` has a text in every language. The cells keep
what the device and bacnetio wrote (states, actions), as in the CSV.

## Functions and where they live

Nothing moves or goes away; each CSV stays where it is. New rows for `docs/concept.md` section 3.3:
the five Excel exports, each tested by opening the file again in a Rust test (sheets, header, a
number cell as a number) and by a browser test that downloads it.

## Compatibility

- CSV files unchanged, column heads included.
- New routes only (`docs/api.md`); `CHANGELOG.md` under *Added*.
- Windows build: `rust_xlsxwriter` needs no system library, so the Windows and Android builds stay
  as they are.

## Order of building

Built on 7 November 2026 in one step, all five.

## Decisions

1. **Library:** `rust_xlsxwriter` or an own minimal writer. *Recommended: `rust_xlsxwriter`.*
2. **Column heads:** in the interface's language, or English as in the CSV. *Recommended: the
   interface's language*, since the file is for people and the CSV remains for scripts.
3. **Which exports:** all five above, or first only the device list, the points and the whole
   site. *Recommended: all five*, in the two steps above.
