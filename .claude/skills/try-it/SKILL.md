---
name: try-it
description: "Set up a bacnetio for the maintainer to try a branch in the browser on this machine (http://127.0.0.1:18081 with the simulators and plugin files in ~/Downloads), keep it running while other work goes on, and switch or stop it. Use when the maintainer wants to test, try, see or click through a change themselves (\"kann ich das testen?\", \"zeig mal\"), or a running demo has to move to another branch."
---

# A bacnetio for the maintainer to try

The maintainer tries changes in their own browser while sessions keep working. Their page must not
change under them.

## The trap

A debug build serves `web/` from the disk (rust-embed reads the folder at run time). Switching the
branch in the folder that serves the maintainer's page changes that page at the next reload, half
built. So the demo runs from a worktree of its own, and work on other topics happens in other
worktrees:

```sh
git worktree add ../bacnetio-demo <branch>      # or switch that worktree when asked
cd ../bacnetio-demo && cargo build --all-targets && node plugins/build.mjs
```

## Start

Outside `unshare`, so the maintainer's browser reaches it; on loopback, so nothing reaches a plant:

```sh
target/debug/examples/simulated_plant 127.0.0.2 47809 &      # AHU-01
target/debug/examples/simulated_controller 127.0.0.3 47809 & # DDC4000_#21, path names
target/debug/examples/simulated_kbob 127.0.0.4 47809 &       # meets KBOB AS-CH
target/debug/bacnetio --no-browser --test-loopback --web-port 18081 --data-dir <scratchpad>/live/data &
cp target/plugins/*.bacnetio-plugin.html ~/Downloads/
```

Run each as a background task and keep its id, to stop exactly that one later (never `pkill -f`).

## Tell the maintainer (German, short)

- the address and to reload the page
- the search: *Ändern*, subnet `127.0.0.0/29`, port `47809`, *Mit diesen Einstellungen suchen*
- plugins: *Einstellungen → Erweiterungen → Erweiterung installieren …*, the file in `~/Downloads`
- what to look at, in a few points

## Keep in mind

- The data folder keeps held values, sites and plugin results over restarts: a value written in a
  demo days ago comes back as held. Say where it comes from (`writes.jsonl`) before anything else.
- The program is in memory: after a rebuild, stop and start bacnetio again; the simulators only when
  they changed.
- Browser tests run in `unshare` (skill browser-tests) and do not collide with this demo; without
  it they would.
