---
name: simulator
description: "Add or extend a simulated BACnet device in bacnetio's examples/ or in a Rust test so that a behaviour seen on a real plant (no segmentation, small APDU, path names, many objects, alarms, schedules, refused services, another program on the port) has a test without a real device. Use when fixing a field problem, when a test needs data no simulator has, or when a browser test needs another device."
---

# Simulators in examples/

Tests never touch real devices (AGENTS.md). Every behaviour from the field gets a simulator that
shows it, then a test against it.

## The simulators

- `simulated_plant.rs`: AHU-01 (1001), mixed object types, trend log, schedule
- `simulated_controller.rs`: a controller named by paths like a DDC4000, 170+ points, no segmentation
- `simulated_kbob.rs` (with the KBOB check plugin): data for that check

The current list is `examples/`; README.md describes each one.

Each takes `<IP> [port]`, binds to its own address (`share_port_by_address`), so several share one
host and port; the browser tests run them on `127.0.0.2` / `127.0.0.3`, port 47809
(`tests/ui/helpers.mjs`). Rust tests build devices in code with `tests/common/mod.rs`
(`simulated_device`), each test on its own ports.

## Steps

1. Prefer a device built inside the Rust test (`tests/common`) when only that test needs it.
2. Extend an example when the browser tests or the maintainer's demos need it; a new example only
   for a device that differs in kind.
3. Make the behaviour explicit and commented: `Segmentation::NONE`, a small max APDU, objects out of
   service, state texts, long names, a service the device refuses.
4. Names and numbers are made up; never copy a real site's names or addresses.
5. A new example is described in `README.md` (`tests/docs.rs` fails otherwise).
6. Update tests that count objects of the example (`tests/ui/*.test.mjs` match names and counts).
7. Run the full check set (skill browser-tests).
