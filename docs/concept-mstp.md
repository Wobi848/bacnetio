# Concept: BACnet MS/TP over a USB RS-485 adapter

Approved on 7 November 2026 (branch `concept-mstp`). Steps 1 to 4 of the building order are built; the field test is open.

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

1. **Client over `AnyTransport`.** Nothing visible changes, all tests stay green. Built on
   7 November 2026 (`src/connection.rs`).
2. **Serial ports and listening**: `GET /api/serial-ports`, the monitor, speed detection,
   simulator `simulated_mstp`. The page shows the result of listening. Built on 7 November 2026
   (`src/mstp.rs`, `src/server/serial.rs`, `web/mstp.js`, `examples/simulated_mstp.rs`). A
   pseudo-terminal has no speed of its own, so the simulator is heard at the first speed tried;
   the choice of speed is tested in `tests/mstp.rs`. The maintainer's CH343 opens and listens at
   every speed.
3. **Join and search**: the address type for IP and MS/TP, the transport in the client, Who-Is,
   the device table with MACs, the log entry, address refusal. The address type comes here, with
   the first MS/TP device, not before. Built on 7 November 2026 (`src/address.rs`,
   `connection::open_mstp`, `src/server/serial.rs`, `web/mstp-actions.js`). bacnetio waits after
   joining until it has held the token (passed it on, or took it alone on a quiet trunk), up to
   20 s, then sends Who-Is, which waits 5 s on MS/TP. Joining and leaving are logged as commands
   under device 4194303. *Device by IP …* is not offered on a trunk; BBMD tables are refused for an
   MS/TP device.
4. **Everything else on MS/TP**: points, writing, recording, exports and plugins checked one by
   one against the simulator. Done on 7 November 2026: reading points, writing and releasing, COV
   reports (`tests/mstp.rs`), the device list and its CSV with `MS/TP 5`, writing in the page and
   leaving the trunk while a value is held, which asks first (`ui/38-mstp-join`). Recording polls
   and plugins read through the same client calls by device ID, which the trunk serves like IP.
   Closing a connection now waits up to 5 s for the requests under way: on MS/TP one is often
   waiting for the token, and leaving failed with "a request is still running".
5. **Field test** with the maintainer's adapter, then fixes.

After step 4 (7 November 2026): a value held over a trunk, after a restart. Releasing it asks to
listen and join first (`409`, `session::other_link`), because bacnetio joins a trunk only after
listening, never by itself; on the trunk it is released as on IP. *Held by you* shows the station of a
value on a trunk. The simulator clears the exclusive mark a killed bacnetio leaves on its pseudo-terminal,
as the system does for a real adapter (`ui/39-mstp-held`).

## Decided (7 November 2026)

1. **Android:** left out for now. Windows and Linux first.
2. **Joining:** listen first, then join after a confirmation in the page, without the unlock. The
   log records joining and leaving.
3. **Own address proposed:** first decided as the highest free one. Changed on 7 November 2026
   after the test: a master polls the addresses after its own one by one, in a ring only one
   every 50 token passes, so at 127 bacnetio waited for minutes, right above a station half a
   second. Now the first free address right above the highest station heard, up to Max_Master.
4. **One connection at a time:** BACnet/IP or MS/TP, switched in *Search settings*. A trunk
   behind a router is already reached over IP.
5. **The maintainer's adapter:** a QinHeng CH343 (USB `1a86:55d3`), on Linux `/dev/ttyACM0`
   (driver `cdc_acm`). No MS/TP devices at hand: the adapter checks opening and naming the port,
   a real trunk waits for the field test.
