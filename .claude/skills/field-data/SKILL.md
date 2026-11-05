---
name: field-data
description: "Evaluate what the maintainer brings back from a real BACnet plant (screenshots, point list CSV, EDE file, request log, console output, error messages like \"abort reason=10\") against docs/field-test.md, find the cause of each problem and turn it into a fix with a simulator test. Use whenever the maintainer sends results from a site, a VPN session or another BACnet tool (K&P browser, YABE), or says something is slow, missing or wrong on a real device."
---

# Evaluating results from a real plant

The maintainer tests on real plants (for example a Kieback&Peter DDC4000 over a HOOC VPN). The
data may hold real addresses and names: never copy them into the repo, tests, issues or commits.
Use made-up names and the simulators instead.

## Steps

1. Match each finding to its row in `docs/field-test.md`: what was expected, what happened.
2. Read the evidence before guessing:
   - **Request log** (device details, *Show request log*): which requests failed or took long,
     timeouts (`abort reason=10` is a TSM timeout: no answer), sizes before a failure
   - **Console window** of bacnetio: port taken, notes, warnings
   - **CSV or EDE**: object counts per type compared with another tool's count; which types miss
   - **Screenshots**: adapter chosen, notes shown, what is cut off
3. Name the cause in one sentence, with the code that explains it (file and line).
4. Ask only for what is missing, specifically: which screen, which file, which button.
5. Turn each cause into a fix with a test against a simulator in `examples/` that shows the same
   behaviour (no segmentation, small APDU, path names, many objects, another program on the port).
   Extend a simulator when none behaves like the plant.
6. Update `docs/field-test.md` with what is proven now and what still needs the plant.

## Known causes from earlier field tests

- Another BACnet program (a browser running as a Windows service) bound to the adapter's address on
  47808 takes the devices' answers: bacnetio must move to another port and say so.
- VirtualBox or VPN adapters called "Ethernet 2" on Windows: the description tells them apart.
- Devices without segmentation: large answers time out instead of being refused; requests must be
  sized from the device's limits.

Answer the maintainer in German.
