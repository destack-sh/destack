#!/usr/bin/env bash
# fetch typescript worker libs into builtin/lib/worker

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

source "$SCRIPT_DIR/versions.sh"
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}
source "$SCRIPT_DIR/fetch-util.sh"

echo "  - worker"
fetch_ts_lib "lib.webworker.d.ts" "$SRC_DIR/worker/index.d.ds"
fetch_ts_lib "lib.webworker.iterable.d.ts" "$SRC_DIR/worker/iterable.d.ds"
fetch_ts_lib "lib.webworker.asynciterable.d.ts" "$SRC_DIR/worker/asynciterable.d.ds"
fetch_ts_lib "lib.webworker.importscripts.d.ts" "$SRC_DIR/worker/importscripts.d.ds"
