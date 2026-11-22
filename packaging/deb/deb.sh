#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/../.."

{ command -v cargo-deb >/dev/null 2>&1 || [ -x "${CARGO_HOME:-$HOME/.cargo}/bin/cargo-deb" ]; } || { echo "deb: cargo-deb not found — cargo install cargo-deb" >&2; exit 1; }
command -v dpkg-shlibdeps >/dev/null 2>&1 || { echo "deb: dpkg-dev not found (needed for \$auto Depends) — install dpkg-dev" >&2; exit 1; }
[ -f target/packaging/app ] || { echo "deb: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }
[ -f icons/eso-addons-manager.png ] || { echo "deb: icons/eso-addons-manager.png not found — run ./icons.sh and commit icons/" >&2; exit 1; }

out=target/dist/deb
mkdir -p "$out"

cargo deb --no-build

for pkg in target/debian/*.deb; do
    [ -f "$pkg" ] || continue
    cp -f "$pkg" "$out/"
    echo "$out/$(basename "$pkg")"
done
