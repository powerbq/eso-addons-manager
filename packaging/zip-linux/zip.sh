#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/../.."

command -v zip >/dev/null 2>&1 || { echo "zip-linux: required tool 'zip' not found" >&2; exit 1; }
[ -f target/packaging/app ] || { echo "zip-linux: target/packaging/app not found — build it first (./build.sh)" >&2; exit 1; }

asset=eso-addons-manager-linux-amd64.zip
out=target/dist/zip-linux

rm -rf "$out"
mkdir -p "$out/stage"
cp target/packaging/app "$out/stage/app"
( cd "$out/stage" && zip -q -X "../$asset" app )
rm -rf "$out/stage"

echo "$out/$asset"
