#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/../.."

command -v curl >/dev/null 2>&1 || { echo "appimage: required tool 'curl' not found" >&2; exit 1; }
[ -f target/packaging/app ] || { echo "appimage: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }
[ -f icons/eso-addons-manager.png ] || { echo "appimage: icons/eso-addons-manager.png not found — run ./icons.sh and commit icons/" >&2; exit 1; }

asset=eso-addons-manager-linux-amd64.AppImage
out=target/dist/appimage
appdir=target/appimage/AppDir
tool="${APPIMAGETOOL:-target/appimagetool.AppImage}"

rm -rf "$appdir"
mkdir -p \
    "$appdir/usr/bin" \
    "$appdir/usr/share/applications" \
    "$appdir/usr/share/icons/hicolor/256x256/apps" \
    "$out"

cp target/packaging/app "$appdir/usr/bin/eso-addons-manager"
cp assets/linux/eso-addons-manager.desktop "$appdir/eso-addons-manager.desktop"
cp assets/linux/eso-addons-manager.desktop "$appdir/usr/share/applications/eso-addons-manager.desktop"
cp icons/eso-addons-manager.png "$appdir/eso-addons-manager.png"
cp icons/eso-addons-manager.png "$appdir/usr/share/icons/hicolor/256x256/apps/eso-addons-manager.png"

cat > "$appdir/AppRun" <<'APPRUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/eso-addons-manager" "$@"
APPRUN
chmod +x "$appdir/AppRun"

if [ ! -x "$tool" ]; then
    curl -fsSL -o "$tool" \
        https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$tool"
fi

ARCH=x86_64 "$tool" "$appdir" "$out/$asset"
rm -rf target/appimage

echo "$out/$asset"
