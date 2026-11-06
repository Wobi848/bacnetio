<img src="assets/logo.png" alt="" width="96" align="right">

# bacnetio

BACnet browser that finds devices reliably, also without administrator rights, and says in plain
words why when it cannot. Work in progress; see [docs/concept.md](docs/concept.md).

## Download

[Releases](https://github.com/Wobi848/bacnetio/releases/latest): `bacnetio-windows-x86_64.exe`,
`bacnetio-linux-x86_64` and `bacnetio-android.apk`. Nothing to install: start the program, and the
browser opens. On Android, allow installing from this source.

## Usage

`bacnetio` without a command starts the local web server on http://127.0.0.1:8081/ and opens the
browser (`--web-port`, `--no-browser`, `--data-dir` for the write log, recording backup and
references). The interface follows mbusio's: a sidebar with the adapter, the search settings and
the areas and a search across the site, a page head with the one next step, a tree, table and
details side by side, a status bar,
light and dark theme, the system's language (English or German), keys `1`–`6`, `/`, arrows.

- **Workspace**: the tree of the site beside the table of what is chosen in it, and its details. With
  the site chosen, the table is the device list: Who-Is plus every address asked directly; plant subnet (VPN), BBMD, port and a
  device ID range under *Search settings* in the sidebar; routers and their networks, diagnosis in plain words. The chosen
  device's facts (vendor, firmware, clock …) beside the list, its clock set to this computer's
  time. Each device's object count shows from the search on. A device the search does not reach
  opens by its IP address.
- **A device** in the workspace, its points: grouped by plant part where the names are paths (`/21/000/00/00/HZ_Kessel.01/temp`),
  otherwise by kind; search, filters for faults, overrides and changes since a saved reference,
  CSV. The chosen point beside the list: value live (COV where offered), state texts, who holds it
  (priority array), write and release after unlocking, every property, trend logs as chart and CSV.
- **Recording**: chosen values every 5 s to 1 min as charts and CSV, backed up every minute.
- **Site problems**: everything faulty, in alarm, overridden, out of service or offline across the
  devices read, and a search across the site in the sidebar.
- **Watch list**: points across devices with their values live.
- **Held by you**: values you still hold, also after a restart, every write of the session, and
  leaving the site with what you hold released.
- **Exports**: device list and points as CSV, also with priorities; EDE files with state texts, per
  device or for the whole site.
- **Sites**: each site searched is kept, named by you, opened again, saved to and read from a file.
- **Extensions**: plugins installed from one file each under *Settings → Extensions*; each gets a
  page in the sidebar and can mark devices and points in the lists. A plugin reads only, it can
  never write to the plant. Writing a plugin: [docs/plugins.md](docs/plugins.md).

A reload keeps the area, the search result, the open device and the search settings. A changed
adapter address, a VPN coming up or a cable plugged in shows without a restart. Starting bacnetio
a second time opens the one already running. With the mouse, press on a watch or record switch in
the point list and drag over other rows to switch them all at once.

The user interface lives in `web/` (plain HTML, CSS and JavaScript, no build step); the JSON API it
uses is described in [docs/api.md](docs/api.md).

To try it without a plant: `cargo run --example simulated_plant -- <IP>` starts a simulated device
on that address; `simulated_controller` is one named by paths with about a hundred points, and
`simulated_commands` a controller whose outputs are held at several priorities. They have to run
on another machine or network namespace than bacnetio, because two programs on one host cannot
both use port 47808 properly; the browser tests run them on loopback addresses with port 47809.

```text
bacnetio interfaces                   list network adapters (* = used by default)
bacnetio scan                         find BACnet devices
bacnetio scan --no-sweep              Who-Is only, do not ask each address directly
bacnetio scan --csv devices.csv       also save the device list for Excel
bacnetio scan --range 1000-1999       only devices with an ID in this range
bacnetio scan --subnet 192.168.10.0/24  ask this subnet, e.g. a plant behind a VPN
bacnetio read 1001                    read all data points of device 1001
bacnetio read --address 192.168.10.20 read the device at that address
bacnetio read 1001 --csv points.csv   also save the point list for Excel
bacnetio write 1001 ao:1 42.5 --priority 8   write a present value
bacnetio release 1001 ao:1 --priority 8      release it again
```

All commands take `--interface IP` (adapter), `--port` (default 47808), `--wait` (seconds for
Who-Is answers) and `--bbmd IP[:PORT]` (register as foreign device at a BBMD to reach devices in
other subnets).

Over a VPN, Who-Is does not arrive (VPNs do not pass on broadcasts) and the adapter's own subnet is
the VPN's, not the plant's. Choose the VPN adapter with `--interface` and give the plant's subnet
with `--subnet`, or register at the plant's BBMD with `--bbmd`; opening a device by address works
too.

`write` and `release` change one present value (objects as `ai ao av bi bo bv msi mso msv` and the
instance, or a type number). They use the same write log as the UI, so the UI lists values the
command line still holds.

`read` gets the object list and name, value, unit, status and description of every object. If
the device cannot send its object list in one piece, it is read in parts (32 entries per
ReadPropertyMultiple, one by one only without it); if it cannot
handle ReadPropertyMultiple, values are read one by one. The output says when that happened.

`scan` sends a Who-Is broadcast, then asks every address of the subnet directly (up to /22).
Direct answers get through the Windows firewall even when broadcast answers are dropped, which
is the usual reason other tools find nothing without administrator rights.

### New versions

Once a day bacnetio asks GitHub whether a newer release is out and, if so, shows a link to it in
the sidebar. GitHub sees the computer's internet address, nothing else; bacnetio never downloads
or installs anything by itself. *Settings → Version* switches the check off, and then bacnetio
does not go online at all. Without internet, as on most plants, nothing is shown.

## Build

```sh
cargo build --release
cargo test
node --test tests/web/*.test.js   # UI logic, no browser needed
```

To try the UI without a plant, run `cargo run --example simulated_plant` on another machine or in a
network namespace (see Usage).

Windows and Linux binaries of any commit: GitHub Actions → Build → Run workflow, then download the
artifact. A tag `v…` builds them and publishes a release.

Windows binary locally on Linux, without MinGW: install [zig](https://ziglang.org) (e.g.
`pip install ziglang`) and `cargo install cargo-zigbuild`, then

```sh
cargo zigbuild --release --target x86_64-pc-windows-gnu
```

## Android

The app in [android/](android/README.md) runs the same program with the same UI on a phone.

## Protocol stack

BACnet encoding, transactions, segmentation and the BACnet/IP transport come from
[rusty-bacnet](https://github.com/jscott3201/rusty-bacnet) (MIT), pinned to an exact version.
Discovery strategy, fallbacks and diagnosis are bacnetio's own (concept section 5.5).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md); security problems please as described in
[SECURITY.md](SECURITY.md), not as a public issue.
