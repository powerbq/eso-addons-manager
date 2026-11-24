#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/../.."

{ command -v cargo-generate-rpm >/dev/null 2>&1 || [ -x "${CARGO_HOME:-$HOME/.cargo}/bin/cargo-generate-rpm" ]; } || { echo "rpm: cargo-generate-rpm not found — cargo install cargo-generate-rpm" >&2; exit 1; }
[ -f target/packaging/app ] || { echo "rpm: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }
[ -f icons/eso-addons-manager.png ] || { echo "rpm: icons/eso-addons-manager.png not found — run ./icons.sh and commit icons/" >&2; exit 1; }

out=target/dist/rpm
mkdir -p "$out"

version="$(sed -nE 's/^version *= *"([^"]+)".*/\1/p' Cargo.toml | head -1 | sed 's/-/~/')"
cargo generate-rpm --set-metadata "version = \"$version\""

for pkg in target/generate-rpm/*.rpm; do
    [ -f "$pkg" ] || continue
    cp -f "$pkg" "$out/"
    echo "$out/$(basename "$pkg")"
done
