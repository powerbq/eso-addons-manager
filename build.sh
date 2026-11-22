#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

if [ -n "${1:-}" ]; then
  export RELEASE_TAG="$1"
fi

cargo build --locked --profile packaging

echo "target/packaging/app"
