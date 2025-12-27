#!/usr/bin/env bash
# fetch typescript dom libs into builtin/lib/dom

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

TS_VERSION=${TS_VERSION:-latest}
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}

echo "  - dom"
mkdir -p "$SRC_DIR/dom"
curl -fsSL "$BASE_URL/lib.dom.d.ts" > "$SRC_DIR/dom/index.d.ds"
curl -fsSL "$BASE_URL/lib.dom.iterable.d.ts" > "$SRC_DIR/dom/iterable.d.ds"
curl -fsSL "$BASE_URL/lib.dom.asynciterable.d.ts" > "$SRC_DIR/dom/asynciterable.d.ds"
