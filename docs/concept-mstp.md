# Concept: BACnet MS/TP over a USB RS-485 adapter

Proposal, 7 November 2026, branch `concept-mstp`. Nothing is built.

## Why

Many field devices sit on MS/TP trunks. Today bacnetio only reaches them through a BACnet router
on IP. Without a router, or when the router is the problem, the technician needs a second tool
(YABE with an MS/TP port). The maintainer wants this as a topic of its own (7 November 2026: "jo
das wäre ein thema"). `docs/concept.md` section 4 lists it under *Later or never*.

## What the library already has

`bacnet-transport` 0.15, feature `serial` (tokio-serial):

- `MstpTransport`: a master node after Clause 9. It takes the token, polls for masters, uses
  extended frames on request, and allows for the latency of USB adapters.
- `TokioSerialPort`: the serial port. A USB adapter switches direction on its own (FTDI, CH340,
  CP2102). `available_port_infos()` lists the ports and says which are USB.
- `mstp::monitor`: listens to a trunk **without joining it** and counts frames and errors per
  station.
- `MstpDiagnostics`: counters and the node's place in the token ring.
- `LoopbackSerial`: an in-memory port for tests.
- `BACnetClient::generic_builder()` takes any transport. `AnyTransport::Mstp` exists.

So bacnetio does not implement Clause 9 itself; it only needs to connect it.

## What has to change in bacnetio

Today every device address is an IP address (`SocketAddrV4` in `scan`, `storage`, `server`). The
client type is `BACnetClient<BipTransport>`. Changes:

1. **One address type:** an IP device or an MS/TP station. Files keep IP addresses as text
   (`10.0.0.20:47808`); an MS/TP station is written `mstp:12`. Old files load unchanged.
2. **The client over either transport** (`AnyTransport`). BBMD, foreign device registration, port
   choice and the sweep of IP addresses stay IP-only and are not offered on MS/TP.
3. **A site on MS/TP** is named after its port and speed until the user names it
   (`COM3 · 38400`).

## Proposal: what the user sees

### Choose the connection

*Search settings* gets the connection type first, as a segmented control:

```text
┌ Search settings ─────────────────────────────────────────┐
│ Connection   [ BACnet/IP ] [ MS/TP (RS-485) ]           │
│                                                          │
│ Port         [ COM3 · USB Serial (FTDI)          ▾ ]     │
│ Speed        [ Find out by listening             ▾ ]     │
│ Own address  [ 127                                ]      │
│              Free on this trunk: 1–4, 9–127. Found: 5,   │
│              6, 7, 8.                                    │
│                                                          │
│                  [ Cancel ] [ Listen to the trunk ]      │
└──────────────────────────────────────────────────────────┘
```

- **Port:** only serial ports. USB ones come first, with the name the system gives them. No
  adapter plugged in: a sentence saying so, and *Look again*.
- **Speed:** 9600, 19200, 38400, 76800, 115200, or *Find out by listening* (the default).
- **Own address:** bacnetio's MAC address on the trunk. A free one is proposed after listening.

### Step 1: listen (changes nothing on the trunk)

*Listen to the trunk* opens the port receive-only (`mstp::monitor`). For each speed, bacnetio
listens about two seconds until valid frames arrive. Then it shows:

- the speed found, or "No frames at any speed: check wiring A/B, termination and the adapter"
- the stations heard passing the token (5, 6, 7, 8) and the highest address polled
- errors in words: "Many damaged frames at 38400: possibly wrong speed, wiring or no
  termination."

Listening is a diagnosis of its own. It helps even when bacnetio never joins.

### Step 2: join and search

*Join the trunk and search* is the next step, with a sentence: "bacnetio joins the token ring as
station 127 at 38400. A station already using 127 would disturb the trunk." Then:

- bacnetio enters the ring and sends Who-Is
- devices appear in the same table as on IP, with the MAC instead of the IP address
- objects, points, writing, recording and plugins work as on IP

*Leave the trunk* (in the sidebar head, beside the site) closes the port. Leaving bacnetio leaves
it too.

### On a phone

The Android app gets no MS/TP for now: an Android phone has no serial port, only USB host access
with a driver of its own per adapter chip. See *Open for the decision*.

## Joining a trunk is an intervention

Reading on IP disturbs no one. On MS/TP, a station in the token ring takes part in the bus timing.
Joining with the wrong speed or an address in use disturbs every device on that trunk. So:

- bacnetio always listens first. *Join* is offered only after a speed was found.
- An address that answered while listening cannot be chosen. The server refuses it as well, not
  only the page.
- Joining asks in the page, as above. It needs no unlock, because it writes no value. The log gets
  an entry: joined and left, port, speed, address.
- Writing values stays as it is: unlock, confirmation, log.
- When the token is lost or the ring breaks after joining, bacnetio says so in words and offers
  *Leave the trunk*.

## Platforms

| Platform | Port | Rights |
| --- | --- | --- |
| Windows | `COM3` | none; the adapter's driver comes with Windows or the vendor |
| Linux | `/dev/ttyUSB0`, `/dev/ttyACM0` | the user must be in the group `dialout` once (administrator). Without it bacnetio says so with the command to type |
| Android | – | not in this step |

## Functions and where they live

New rows for `docs/concept.md` section 3.3. No existing function moves or goes away.

| Function | Place | Test |
| --- | --- | --- |
| Choose BACnet/IP or MS/TP | *Search settings*, first row | `ui/` new, `web/ui.test.js` |
| List serial ports, USB first | *Search settings* | `serial.rs` (list sorted), `ui/` new |
| Listen to a trunk: speed, stations heard, errors | *Search settings*, *Listen to the trunk* | `mstp.rs` with `LoopbackSerial`, simulator |
| Join, search, leave | next step; sidebar head | simulator `simulated_mstp`, `ui/` new |
| Refuse an own address already in use | server and page | `mstp.rs`, `ui/` new |
| Log entry for joining and leaving | *Written* log | `storage.rs` |
| Devices with MAC instead of IP | table of the site, device details, CSV | `scan.rs`, `ui/13-exports` |

For the functions that stay IP-only (BBMD, foreign device, *Device by IP …*, port, sweep), the
search settings show them only with BACnet/IP chosen. Their rows in 3.3 get the note "BACnet/IP".

## Tests without hardware

- Rust tests: `LoopbackSerial` between the client and a server from `bacnet-server`.
- Simulator `examples/simulated_mstp`: a device with a few objects on a pseudo-terminal (Linux).
  It prints the port's name (`/dev/pts/7`), and bacnetio opens it like an adapter. With this the
  browser tests run on CI (Linux). Windows has no pseudo-terminals; there the Rust tests cover
  the logic.
- A real adapter and a real trunk only in the maintainer's field test. `docs/field-test.md` gets
  a section on MS/TP.

## Compatibility

- Saved sites, logs, recordings and settings keep loading: IP addresses stay as they are written.
- The web API only gains fields and routes: `link` in the search settings (default `ip`),
  `GET /api/serial-ports`, `POST /api/mstp/listen`, `POST /api/mstp/leave`. Described in
  `docs/api.md`.
- The plugin API: a device gets `address` as text as before; on MS/TP it is `mstp:12`. Noted in
  `docs/plugins.md`.
- `CHANGELOG.md`: "MS/TP directly over a USB RS-485 adapter (Windows, Linux)".

## Order of building

Each step leaves everything working and is its own pull request:

1. **Address type** for IP and MS/TP, client over `AnyTransport`. Nothing visible changes, all
   tests stay green.
2. **Serial ports and listening**: `GET /api/serial-ports`, the monitor, speed detection,
   simulator `simulated_mstp`. The page shows the result of listening.
3. **Join and search**: transport in the client, Who-Is, the device table with MACs, the log
   entry, address refusal.
4. **Everything else on MS/TP**: points, writing, recording, exports and plugins checked one by
   one against the simulator.
5. **Field test** with the maintainer's adapter, then fixes.

## Open for the decision

1. **Android:** leave it out for now? Recommendation: yes. Windows and Linux first. Android needs
   a USB serial driver per chip (FTDI, CH340, CP2102) in the app, a project of its own.
2. **Joining:** listen first, then join with a confirmation in the page, no unlock. Is that enough,
   or should joining need the unlock like writing? Recommendation: confirmation without unlock,
   because joining writes no value. The log records it.
3. **Own address proposed:** the highest free one (often 127), or the lowest free above the
   stations found? Recommendation: the highest free one up to the Max_Master heard while
   listening. Devices are usually numbered from below, so the highest address collides least.
4. **IP and MS/TP at once**, or one connection at a time? Recommendation: one at a time. A switch
   in *Search settings*. Two at once means two sites in one view and doubles every state.
5. **Your adapter:** which one do you have (chip, Windows or Linux)? It decides what the field
   test checks first.
