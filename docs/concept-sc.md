# Concept: BACnet/SC (Secure Connect)

Status: approved on 09.11.2026, decisions below. Steps 0 and 1 built on 10.11.2026 (branches `sc-build`, `sc-connection`); nothing to see yet. The maintainer asked for the
concept in full and whether it works with bacnetio's interface to BACnet ("bitte nichts bauen zu SC
aber Konzept voll ausbauen und ob es geht mit unserer Schnittstelle").

## Why

BACnet/SC (ASHRAE 135-2020, Annex AB) carries BACnet over TLS-encrypted WebSockets instead of
plain UDP. New plants use it where the owner's IT asks for encryption and for no broadcasts on its
network. The maintainer has no SC plant yet ("man weiss nie"), but people who use bacnetio may, and
a BACnet browser that cannot open such a plant is of no use there. With users on SC plants,
bacnetio can be tested and developed further on real ones.

## BACnet/SC in short

- **Hub and spokes.** Every node (device, workstation, bacnetio) keeps one WebSocket connection
  (`wss://`) to the site's **hub**. The hub passes messages between the nodes and copies
  broadcasts (Who-Is, I-Am) to all of them. There may be a **failover hub**.
- **Certificates on both sides.** TLS 1.3 with a client certificate: the hub accepts a node only
  if its **operational certificate** is signed by the site's certificate authority (CA), and the
  node accepts the hub the same way. The site's CA is usually run by the integrator or the
  vendor's tool. Without a certificate from it, nothing connects; that is the point of SC.
- **VMAC.** Each node has a 6-octet virtual MAC address on the SC network, chosen at random, and a
  **device UUID** that it keeps for its lifetime.
- **Direct connections** (node to node, without the hub) are optional in the standard.
- BACnet itself is unchanged above the link: Who-Is, ReadProperty, WriteProperty, COV, ReadRange,
  alarms, routers to other networks.

## Does it work with bacnetio's interface? Yes: tested on 09.11.2026

bacnetio talks BACnet through `bacnet-client` 0.15 and opens every connection as
`BACnetClient<AnyTransport>` (`src/connection.rs`): BACnet/IP and MS/TP are two kinds of
`AnyTransport` today. The library has SC behind the feature `sc-tls` (`bacnet-client`,
`bacnet-server`, `bacnet-transport`):

- `AnyTransport::Sc`: SC as a third kind of the same transport, so the client type, and with it
  everything built on it (search, reading, writing, COV, trend logs, alarms), stays as it is.
- `TlsWebSocket::connect(url, ScNodeTlsConfig)`: the connection to a hub; `ScNodeTlsConfig` from
  the CA certificate, bacnetio's certificate and its private key (DER).
- `ScTransport`: VMAC, device UUID, heartbeat, reconnect, failover hub.
- `ScHub` and `BACnetServer::sc_builder()`: a hub and an SC device in the same process, for a
  simulator and for tests without a plant.

**Tried outside the repository** (a test program in the session's scratch folder, not committed):
certificates made on the spot (a CA, and one certificate each for hub, device and client), a hub on
`127.0.0.1`, a simulated SC device with an analog input on it, and a client built the way bacnetio
builds its clients (`generic_builder().transport(AnyTransport::Sc(..))`). Result: Who-Is through
the hub found the device (VMAC `20:00:00:00:00:01`), and ReadProperty read the input's name over
SC. It compiled for Linux at the first try.

**Platforms:**

| Platform | Result |
| --- | --- |
| Linux | built and run (the test above) |
| Android (aarch64) | built with the NDK, with the same patch of `bacnet-transport` that `android/build-core.sh` applies today; the APK built in CI on 10.11.2026; not run on a phone |
| Windows | release build in CI on 10.11.2026 (step 0, run of the Build workflow on `sc-build`); not run against a hub |

The TLS stack makes bacnetio larger and its build slower (it compiles C code). Measured in step 0
on 10.11.2026: the files of release v0.3.0 against the same build with `sc-tls` from CI, and the
Linux release build from scratch on the maintainer's machine:

| | without `sc-tls` | with `sc-tls` |
| --- | --- | --- |
| Windows exe | 11.6 MB | 14.2 MB (+2.7 MB) |
| Linux program | 9.6 MB | 10.7 MB (+1.1 MB) |
| Android APK | 6.6 MB | 7.1 MB (+0.5 MB) |
| Linux build time | 97 s | 129 s (+32 s) |

## What has to change in bacnetio

1. **Addresses.** A VMAC has six octets, exactly like a BACnet/IP address (four of IP, two of
   port). `Address::from_mac` (`src/address.rs`) guesses the kind from the length today, so it
   would take every SC device for an IP address. The connection must say how to read a MAC:
   `Address::from_mac(link, mac)`, with a third kind `Address::Sc([u8; 6])`, written
   `sc:20:00:00:00:00:01` in files and shown `SC 20:00:00:00:00:01`. Old files load unchanged, as
   when MS/TP came (`docs/concept-mstp.md`).
2. **The connection.** `connection::open_sc(hub, certificates)` beside `open_ip` and
   `open_mstp`: the transport into the same `Client`. BBMD, foreign device registration, the port
   choice and the IP sweep are not offered on SC, as on MS/TP.
3. **bacnetio's own identity on SC:** a random VMAC and device UUID made once and kept in the data
   folder, so the hub knows bacnetio again after a restart (the standard asks for a UUID that never
   changes).
4. **Certificates** kept in the data folder, private key only there (see *Keys and certificates*).
5. **Where a device was reached:** a value held on an SC device is released only while bacnetio is
   on SC, as for MS/TP today ("a device on an MS/TP trunk is written and released only while
   bacnetio is on a trunk").

## Proposal: what the user sees

### Choose the connection

*Search settings* get a third connection type beside *BACnet/IP* and *MS/TP (RS-485)*:

```text
┌ Search settings ──────────────────────────────────────────────┐
│ Connection   [ BACnet/IP ] [ MS/TP (RS-485) ] [ BACnet/SC ]   │
│                                                               │
│ Hub          [ wss://hub.plant.example:443/bacnet         ]   │
│ Failover hub [                                  (optional) ]   │
│                                                               │
│ Certificates of this site                                     │
│  Site CA            ca.pem          ✓ valid until 03.2031     │
│                     [ Choose file … ]                         │
│  bacnetio's own     bacnetio.pem    ✓ valid until 11.2027     │
│                     [ Choose file … ]                         │
│  Private key        bacnetio.key    ✓ matches the certificate │
│                     [ Choose file … ]                         │
│                                                               │
│                 [ Cancel ] [ Connect to the hub and search ]  │
└───────────────────────────────────────────────────────────────┘
```

- **Hub:** the address the integrator gives, `wss://` only. The failover hub is optional.
- **Certificates:** in whatever form integrators and vendor tools hand them out: PEM or DER files
  (`.pem`, `.crt`, `.cer`, `.der`, `.key`), a PEM file holding certificate and key together, or one
  PKCS#12 file (`.p12`, `.pfx`) with certificate, key and CA, which then fills all three lines at
  once. Each is checked when chosen, and the line says what was found in words: valid until, not
  yet valid, expired, not signed by this CA, key does not match the certificate, not a certificate
  at all. A certificate that runs out within 30 days says so in orange.
- **Password:** an encrypted key (PKCS#8) or a PKCS#12 file asks for its password in the page. The
  password opens the key and is then forgotten; it is never stored or logged. See *Keys and
  certificates* for what is kept.
- **Make a key here** (in its own step): bacnetio makes a key pair and a certificate request (CSR)
  for this site, saves the request as a file for the integrator, and takes the signed certificate
  back with *Choose file …*. The key never leaves the computer.

### Connect and search

*Connect to the hub and search* opens the connection and sends Who-Is through the hub. The
devices appear in the same table as on IP, with `SC 20:00:00:00:00:01` as their address. Points,
writing, recording, trend logs, alarms, schedules, files and plugins work as on IP.

When the connection fails, the reason in words, not a TLS error code:

| What happens | What bacnetio says |
| --- | --- |
| Name or address of the hub not found, nothing answers | "No hub answers at this address: check the address and that this computer reaches it (VPN, firewall, port 443)." |
| The hub's certificate is not from this site's CA | "The hub's certificate is not signed by the site CA you chose: wrong CA file, or not the hub of this site." |
| The hub refuses bacnetio's certificate | "The hub refuses bacnetio's certificate: it is not from this site's CA, has run out, or the hub does not admit it yet." |
| Certificate expired or not yet valid | "bacnetio's certificate is valid from … to …: ask the integrator for a new one." |
| The hub drops the connection later | the status bar says so and bacnetio connects again by itself (the library's reconnect), to the failover hub if there is one |

The status bar shows `BACnet/SC · hub.plant.example · connected` (or *connecting*, *failover hub*).
*Leave the SC network* in the sidebar head closes the connection, as *Leave the trunk* does.

### A site on SC

A site found on SC is named after its hub (`hub.plant.example`) until the user names it, and it
remembers the hub address and which certificates belong to it (by their fingerprints, never the
key), so opening it again offers *Connect to the hub* with the same files.

### On a phone

The Android app gets SC as well: it needs no hardware, only files. Choosing the certificate files
uses the system's file picker. Built and checked on a phone in its own step.

## Keys and certificates

The private key is the one secret in this concept. It lets whoever holds it into the plant's
network.

- Kept by default only while bacnetio runs: a key that came with a password is asked for again at
  the next start. *Remember the key on this computer* (off by default) keeps it in bacnetio's data
  folder (`sc/` below `--data-dir`) without the password, readable only for the user: file mode
  0600 on Linux and Android, and on Windows a file in the user's profile, encrypted for that user
  with the system's data protection (DPAPI). A key that came without a password is kept the same
  way, as the file it came from was not protected either. It is never in a
  site file, an export, the Excel files, the log, the capture, the plugin API, the web API's
  answers or the console.
- The certificates themselves are not secret. A site file may carry the CA certificate and
  bacnetio's certificate, never the key; opening such a site on another computer asks for the key
  file there.
- *Forget the SC key* in *Settings* deletes the key file. Leaving the SC network keeps it.
- bacnetio never makes the plant's CA, signs certificates or changes a hub's admission list.
- The web API takes the files as uploads from the local page only (the `Host`/`Origin` guard covers
  the routes, as every route).

## What stays out

- **bacnetio as a hub:** a site has its hub; bacnetio connects to it.
- **The capture for Wireshark on SC:** the capture records BACnet/IP and MS/TP frames today. On SC
  the frames travel inside TLS. bacnetio could write the BACnet messages it sends and receives
  without TLS, but Wireshark reads BACnet/SC only from WebSocket traffic. *Capture* stays
  unavailable on SC and says why; a later step can look for a format Wireshark reads.

## Writing to plants

Unchanged: writing stays behind the session unlock with the confirmation in the page and the log
entry. The log keeps the SC address of the device (`sc:…`), and *Held by you* and the leaving
check list values held on SC devices like all others. Releasing one asks for the SC connection
when bacnetio is on another one.

## Functions and where they live

New rows for `docs/concept.md` section 3.3 when built. No existing function moves or goes away.

| Function | Place | Test |
| --- | --- | --- |
| Connect to an SC hub, with failover | *Search settings* → *BACnet/SC* | Rust test with `ScHub` and an SC device; browser test against `simulated_sc` |
| Certificates chosen and checked in words | *Search settings* → *BACnet/SC* | unit tests with certificates made in the test |
| Search, read, write, COV, trend logs, alarms on SC | as on IP | browser tests on `simulated_sc` |
| Held values on SC devices, released only on SC | *Held by you*, leaving check | Rust test like the MS/TP one |
| Status of the hub connection | status bar | browser test |
| Leave the SC network | sidebar head | browser test |
| Forget the SC key | *Settings* | browser test |

`examples/simulated_sc.rs`: a hub and an SC device with points, a trend log and alarms, on
`127.0.0.1`, with certificates it makes at start and writes as PEM files into a folder it names,
for the demo and the browser tests. No key or certificate enters the repository.

## Compatibility

- Files: `writes.jsonl`, sites and settings keep loading; the SC address is a new form beside
  `10.0.0.20:47808` and `mstp:12`. An older bacnetio counts a line with an SC address as
  unreadable and goes on, as it does for any line it cannot read.
- The web API gets new fields and routes, none renamed.
- The program grows by the TLS stack, 1 to 3 MB depending on the platform (step 0). `CHANGELOG.md` names SC when it is
  built.

## Order of building

0. ✓ **Build check, nothing to see** (done 10.11.2026, all three build): the feature `sc-tls` on in `Cargo.toml`, CI builds Windows,
   Linux and Android; size and build time measured. If Windows fails, stop and look again.
1. ✓ **Addresses and connection** (done 10.11.2026): `Address::Sc` (`src/address.rs`), `from_mac`
   by the kind of link (`Link`, `connection::link`), `connection::open_sc`, the VMAC and UUID kept
   in `sc/identity.json` (`src/sc_identity.rs`); a device is released only over the link it was
   written on, SC included (`other_link` in `src/server/session.rs`). Test: `tests/sc.rs` with a hub
   and an SC device made in the test, and a certificate of another CA refused.
2. **Search settings and certificates:** the third connection type, the three files checked in
   words, the key kept as described, the reasons for a failed connection; `simulated_sc` and
   browser tests; help guide *Open a BACnet/SC plant* in all languages.
3. **Status, leaving, sites:** status bar, *Leave the SC network*, a site remembers its hub and
   certificates, held values only on SC.
4. **Android:** the file picker, checked on a phone.
5. **Key and CSR made in bacnetio**, the signed certificate taken back.
6. **Direct connections** (Annex AB.4, node to node without the hub), where the hub announces them:
   used when the device offers one, the hub stays the way back. A setting turns them off.

Each step leaves everything else working; the field test on a real SC plant follows when a user
has one.

## Decided on 09.11.2026

The maintainer left the choice to bacnetio's side, with two wishes: secure on every platform, and
as flexible as possible ("sc sicher auf allen plattformen ... es soll möglichst flexibel sein").

1. **Certificates:** every usual form (PEM, DER, PKCS#12) in step 2, and a key with CSR made in
   bacnetio in step 5. Flexible for every way a site hands out certificates.
2. **Keys with a password:** accepted (encrypted PKCS#8, PKCS#12). The password is never kept; the
   key is kept only when the user asks for it, protected as described under *Keys and
   certificates*.
3. **Every build:** Windows, Linux and Android, with the same TLS stack (TLS 1.3, certificates
   checked against the site's CA only, no system certificate store, no way to switch the check off).
   Step 0 proves the build on all three before anything else.
4. **Direct connections:** built as step 6, after the hub connection works on a real plant.

Added on 10.11.2026, asked by the maintainer ("könnten wir am Schluss auch builden ohne SC?"):

5. **A build without SC:** the feature `sc` of bacnetio, on by default, brings the TLS stack;
   `cargo build --no-default-features` leaves it out. The releases keep SC on all three platforms;
   the build without is the way out should the TLS stack ever not build on one. The SC code sits in
   its own modules behind `#[cfg(feature = "sc")]`; an SC address stays readable in every build, so
   files load the same. CI checks the build without (`cargo clippy --no-default-features`).

Reading PKCS#12 and encrypted keys needs a library beside the TLS stack (pure Rust, so it builds
the same on every platform). Chosen on 10.11.2026: `p12-keystore` for PKCS#12, with the old
3DES/RC2 form older Windows and many vendor tools still write, and `pkcs8` for encrypted keys. The
dates, the 7 and 30 days before a certificate runs out, whether the CA signed bacnetio's
certificate and whether the key fits it come from the check `bacnet-transport` brings
(`inspect_sc_pem`); bacnetio hands it the files as PEM.

Step 2 is built in parts: first reading and checking the files and keeping them
(`src/sc_files.rs`, `src/sc_certificates.rs`, `src/sc_store.rs`, with unit tests for every form,
password, expiry and mismatch), then the routes, the search settings and the simulator.
