#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

cargo build --locked --profile packaging

echo "target/packaging/app"
