#!/usr/bin/env bash
# fetch typescript scripthost libs into builtin/lib

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

source "$SCRIPT_DIR/versions.sh"
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}
source "$SCRIPT_DIR/fetch-util.sh"

echo "  - scripthost"
fetch_ts_lib "lib.scripthost.d.ts" "$SRC_DIR/scripthost.d.ds"
