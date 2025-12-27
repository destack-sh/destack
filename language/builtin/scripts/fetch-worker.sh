#!/usr/bin/env bash
# fetch typescript worker libs into builtin/lib/worker

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

TS_VERSION=${TS_VERSION:-latest}
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}

echo "  - worker"
mkdir -p "$SRC_DIR/worker"
curl -fsSL "$BASE_URL/lib.webworker.d.ts" > "$SRC_DIR/worker/index.d.ds"
curl -fsSL "$BASE_URL/lib.webworker.iterable.d.ts" > "$SRC_DIR/worker/iterable.d.ds"
curl -fsSL "$BASE_URL/lib.webworker.asynciterable.d.ts" > "$SRC_DIR/worker/asynciterable.d.ds"
