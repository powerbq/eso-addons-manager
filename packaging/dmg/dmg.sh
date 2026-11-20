#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/../.."

[ -f target/packaging/app ] || { echo "dmg: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }
[ -f assets/macos/Info.plist ] || { echo "dmg: assets/macos/Info.plist not found" >&2; exit 1; }
[ -f icons/eso-addons-manager.icns ] || { echo "dmg: icons/eso-addons-manager.icns not found — run ./icons.sh and commit icons/" >&2; exit 1; }

version="${VERSION:-$(sed -nE 's/^version *= *"([^"]+)".*/\1/p' Cargo.toml | head -1)}"
app="ESO Addons Manager.app"
asset="eso-addons-manager-${version}-macos-arm64.dmg"
out=target/dist/dmg
staging=target/macos/stage

rm -rf target/macos
mkdir -p "$staging/$app/Contents/MacOS" "$staging/$app/Contents/Resources" "$out"

cp assets/macos/Info.plist "$staging/$app/Contents/Info.plist"
sed -i '' -e "s/__VERSION__/$version/g" "$staging/$app/Contents/Info.plist"
cp icons/eso-addons-manager.icns "$staging/$app/Contents/Resources/eso-addons-manager.icns"
cp target/packaging/app "$staging/$app/Contents/MacOS/app"
chmod +x "$staging/$app/Contents/MacOS/app"
ln -s /Applications "$staging/Applications"

rm -f "$out/$asset"
hdiutil create -volname "ESO Addons Manager" -srcfolder "$staging" -ov -format UDZO "$out/$asset"
rm -rf target/macos

echo "$out/$asset"
