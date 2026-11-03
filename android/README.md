# bacnetio for Android

The BACnet browser on a phone or tablet in the plant's Wi-Fi.

## Install

Copy `app-debug.apk` to the phone, open it and allow installing from this source.

## How it works

- The app ships bacnetio itself (Rust, built for `aarch64-linux-android`) as `libbacnetio.so` and
  runs it as a separate process in a foreground service, so recordings keep running with the
  screen off. The UI is the normal web UI in a WebView (`http://127.0.0.1:8081`).
- The service holds a Wi-Fi multicast lock: without it Android drops broadcast packets, and
  devices answering Who-Is by broadcast would not be found. Asking each address directly works
  either way.
- CSV exports are saved to Downloads.
- Same pattern as mbusio for Android: a crash of bacnetio does not take the app down, the service
  starts it again. The log is `bacnetio.log` in the app data.

## Build

Needs the Android SDK with NDK 28.2 and the Rust target `aarch64-linux-android`
(`rustup target add aarch64-linux-android`).

```sh
cd android
ANDROID_HOME=/usr/lib/android-sdk ./gradlew assembleDebug
```

`build-core.sh` builds the Rust part. bacnet-transport 0.15.0 does not compile for Android
(`ipi6_ifindex` is `i32` in Android's libc), so the script builds from a copy of the sources with
a patched copy of that crate; the repository and `Cargo.lock` stay untouched. Drop the patch once
the crate is fixed upstream.
