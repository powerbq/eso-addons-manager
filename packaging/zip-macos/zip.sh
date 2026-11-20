#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/../.."

[ -f target/packaging/app ] || { echo "zip-macos: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }
[ -f assets/macos/Info.plist ] || { echo "zip-macos: assets/macos/Info.plist not found" >&2; exit 1; }
[ -f icons/eso-addons-manager.icns ] || { echo "zip-macos: icons/eso-addons-manager.icns not found — run ./icons.sh and commit icons/" >&2; exit 1; }

version="${VERSION:-$(sed -nE 's/^version *= *"([^"]+)".*/\1/p' Cargo.toml | head -1)}"
asset=eso-addons-manager-macos-arm64.zip
out=target/dist/zip-macos
bundle="$out/app.app"

rm -rf "$out"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp assets/macos/Info.plist "$bundle/Contents/Info.plist"
sed -i '' -e "s/__VERSION__/$version/g" "$bundle/Contents/Info.plist"
cp icons/eso-addons-manager.icns "$bundle/Contents/Resources/eso-addons-manager.icns"
cp target/packaging/app "$bundle/Contents/MacOS/app"
chmod +x "$bundle/Contents/MacOS/app"

ditto -c -k --keepParent "$bundle" "$out/$asset"
rm -rf "$bundle"

echo "$out/$asset"
