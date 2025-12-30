#!/usr/bin/env bash
# fetch typescript dom libs into builtin/lib/dom

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

source "$SCRIPT_DIR/versions.sh"
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}
source "$SCRIPT_DIR/fetch-util.sh"

echo "  - dom"
fetch_ts_lib "lib.dom.d.ts" "$SRC_DIR/dom/index.d.ds"
fetch_ts_lib "lib.dom.iterable.d.ts" "$SRC_DIR/dom/iterable.d.ds"
fetch_ts_lib "lib.dom.asynciterable.d.ts" "$SRC_DIR/dom/asynciterable.d.ds"
