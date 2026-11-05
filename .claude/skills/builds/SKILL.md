---
name: builds
description: "Build bacnetio for Windows (exe from Linux with zig) or Android (APK), only when the maintainer explicitly asks for that build in the current request (\"bau mir die exe\", \"Windows-Version\", \"APK\"). Never offer it on its own initiative and never publish anything."
disable-model-invocation: true
---

# Windows and Android builds of bacnetio

Only on the maintainer's explicit request in this conversation (AGENTS.md: Windows builds only on
explicit request). Building is not publishing: no tag, release, upload or push of the files.

## Windows exe on Linux

Needs zig and cargo-zigbuild, no MinGW:

```sh
pip install ziglang            # or a zig from the package manager; a venv in the scratchpad works
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-pc-windows-gnu
```

The exe is `target/x86_64-pc-windows-gnu/release/bacnetio.exe`. The icon is embedded only when
building on a Windows host (`build.rs`). Copy it where the maintainer asks (usually `~/Downloads`).
Say that SmartScreen warns about an unsigned file and the firewall asks on first start.

## Android APK

See `android/README.md`. In short: `android/build-core.sh` builds the Rust part (it patches a copy
of bacnet-transport for Android, so the repo and `Cargo.lock` stay untouched), then

```sh
cd android && ANDROID_HOME=/usr/lib/android-sdk ./gradlew assembleDebug
```

The APK is under `android/app/build/outputs/apk/debug/`. Signing keys never enter the repo.

## After a build

Say which commit and branch it was built from, and that it is untested on the target unless the
maintainer tried it.
