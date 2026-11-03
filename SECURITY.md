# Security

bacnetio talks to building automation: it can change values, priorities and device settings in a
running plant. A flaw that lets a page, a file or another program make it write, or that exposes
its web interface beyond the computer it runs on, is a security issue.

## Reporting

Please do not open a public issue for a security problem. Use GitHub's private vulnerability
reporting (*Security → Report a vulnerability* on the repository) and describe what you found and
how to reproduce it. The maintainer answers as soon as possible and fixes it before it is made
public.

## What bacnetio does to keep a plant safe

- The web server listens on `127.0.0.1` only and refuses requests that do not name it
  (`src/server/guard.rs`), so other pages in the browser cannot use it.
- Every write needs the unlock of the session, a confirmation, and lands in the write log.
- Values bacnetio holds at a priority are listed until they are released, and leaving asks first.

BACnet itself has no authentication. Anyone on the building network can do what bacnetio does;
keep that network separate from others.
