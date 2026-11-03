#!/bin/sh
# Builds bacnetio for Android (aarch64) and copies the binary to the path given as argument.
#
# bacnet-transport 0.15.0 does not compile for Android: libc declares in6_pktinfo.ipi6_ifindex as
# i32 there, u32 elsewhere. Until that is fixed upstream, this script builds from a copy of the
# sources with a patched copy of the crate, so the repository and its Cargo.lock stay untouched.
set -eu

destination=$1

repo=$(cd "$(dirname "$0")/.." && pwd)
work="$repo/android/build/core"
target="$repo/target"

cargo fetch --manifest-path "$repo/Cargo.toml" --target aarch64-linux-android >&2
transport=$(cargo metadata --manifest-path "$repo/Cargo.toml" --format-version 1 \
    | grep -o '"manifest_path":"[^"]*bacnet-transport-0.15.0/Cargo.toml"' | head -n 1 \
    | sed 's/"manifest_path":"//; s|/Cargo.toml"$||')

rm -rf "$work"
mkdir -p "$work/vendor"
(cd "$repo" && git ls-files -z | tar --null -T - -cf -) | tar -C "$work" -xf -
cp -r "$transport" "$work/vendor/bacnet-transport"
sed -i 's/Some(info\.ipi6_ifindex)))/Some(info.ipi6_ifindex as u32)))/' \
    "$work/vendor/bacnet-transport/src/udp_metadata_nix.rs"
grep -q 'ipi6_ifindex as u32' "$work/vendor/bacnet-transport/src/udp_metadata_nix.rs"
printf '\n[patch.crates-io]\nbacnet-transport = { path = "vendor/bacnet-transport" }\n' >> "$work/Cargo.toml"

CARGO_TARGET_DIR="$target" cargo build --release --target aarch64-linux-android \
    --manifest-path "$work/Cargo.toml" >&2
mkdir -p "$(dirname "$destination")"
cp "$target/aarch64-linux-android/release/bacnetio" "$destination"
