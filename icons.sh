#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

source=""
if [ -f assets/eso-addons-manager.svg ]; then
    source=assets/eso-addons-manager.svg
else
    for candidate in assets/eso-addons-manager.*; do
        case "${candidate##*.}" in
            png | jpg | jpeg | webp | gif | bmp | tif | tiff | avif)
                source="$candidate"
                break
                ;;
        esac
    done
fi

if [ -z "$source" ]; then
    echo "no source icon: expected assets/eso-addons-manager.<svg|png|jpg|webp|...>" >&2
    exit 1
fi

if command -v magick >/dev/null 2>&1; then
    im=(magick)
elif command -v convert >/dev/null 2>&1; then
    im=(convert)
else
    echo "ImageMagick is required (install 'magick' or 'convert')" >&2
    exit 1
fi

render() {
    "${im[@]}" -background none "$source" -resize "${1}x${1}" -strip "$2"
}

mkdir -p icons

render 256 icons/eso-addons-manager.png

"${im[@]}" -background none "$source" \
    -define icon:auto-resize=16,32,48,64,128,256 \
    -strip \
    icons/eso-addons-manager.ico

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
for size in 16 32 64 128 256 512 1024; do
    render "$size" "$work/$size.png"
done

icns=icons/eso-addons-manager.icns
if command -v iconutil >/dev/null 2>&1; then
    iconset="$work/icon.iconset"
    mkdir -p "$iconset"
    cp "$work/16.png"   "$iconset/icon_16x16.png"
    cp "$work/32.png"   "$iconset/icon_16x16@2x.png"
    cp "$work/32.png"   "$iconset/icon_32x32.png"
    cp "$work/64.png"   "$iconset/icon_32x32@2x.png"
    cp "$work/128.png"  "$iconset/icon_128x128.png"
    cp "$work/256.png"  "$iconset/icon_128x128@2x.png"
    cp "$work/256.png"  "$iconset/icon_256x256.png"
    cp "$work/512.png"  "$iconset/icon_256x256@2x.png"
    cp "$work/512.png"  "$iconset/icon_512x512.png"
    cp "$work/1024.png" "$iconset/icon_512x512@2x.png"
    iconutil -c icns -o "$icns" "$iconset"
elif command -v png2icns >/dev/null 2>&1; then
    png2icns "$icns" \
        "$work/16.png" "$work/32.png" "$work/128.png" \
        "$work/256.png" "$work/512.png" "$work/1024.png"
else
    "${im[@]}" "$work/1024.png" -strip "$icns"
fi

echo "regenerated from $source:"
echo "  icons/eso-addons-manager.png"
echo "  icons/eso-addons-manager.ico"
echo "  $icns"
