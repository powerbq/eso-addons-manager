#!/bin/bash

set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

command -v makepkg >/dev/null 2>&1 || { echo "arch: makepkg not found (Arch only)" >&2; exit 1; }
[ -f target/packaging/app ] || { echo "arch: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }
[ -f icons/eso-addons-manager.png ] || { echo "arch: icons/eso-addons-manager.png not found — run ./icons.sh and commit icons/" >&2; exit 1; }

out="$root/target/dist/arch"
mkdir -p "$out" target/arch

cd packaging/arch
PKGDEST="$out" BUILDDIR="$root/target/arch" PKGEXT='.pkg.tar.zst' makepkg -f --nodeps
