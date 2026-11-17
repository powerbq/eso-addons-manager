#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

case "$(uname -s)" in
    Linux)
        asset=eso-addons-manager-linux-amd64.zip
        ;;
    Darwin)
        asset=eso-addons-manager-macos-arm64.zip
        ;;
    *)
        echo "unsupported system: $(uname -s), use build.cmd on Windows" >&2
        exit 1
        ;;
esac

cargo build --locked --profile packaging

rm -rf target/dist
mkdir -p target/dist/package

if [ "$(uname -s)" = Darwin ]; then
    bundle=target/dist/package/app.app
    mkdir -p "$bundle/Contents/MacOS"
    cp assets/macos/Info.plist "$bundle/Contents/Info.plist"
    cp target/packaging/app "$bundle/Contents/MacOS/app"
    ditto -c -k --keepParent "$bundle" "target/dist/$asset"
    checksum=$(shasum -a 256 "target/dist/$asset")
else
    cp target/packaging/app target/dist/package/app
    (cd target/dist/package && zip -q -X "../$asset" app)
    checksum=$(sha256sum "target/dist/$asset")
fi

printf '%s' "${checksum%% *}" > "target/dist/$asset.sha256"
rm -rf target/dist/package

echo "target/dist/$asset"
