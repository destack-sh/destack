#!/usr/bin/env bash
set -euo pipefail

WPT_REF="${WPT_REF:-55d076dca564300616a75eec5ec696e805c7bc3b}"
WEBGPU_CTS_REF="${WEBGPU_CTS_REF:-9726cfe2893834c4bb42b435638c4e7362f4c258}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WPT_TARGET="$SCRIPT_DIR/checkouts/wpt"
CTS_TARGET="$SCRIPT_DIR/checkouts/webgpu-cts"
HELPER="$SCRIPT_DIR/../fetch-fixtures.sh"

mkdir -p "$SCRIPT_DIR/checkouts"

export REPO_URL="https://github.com/web-platform-tests/wpt.git"
bash "$HELPER" "wpt web fixtures" "main" "$WPT_REF" "$WPT_TARGET" \
    "fetch" \
    "url" \
    "streams" \
    "encoding" \
    "FileAPI" \
    "IndexedDB" \
    "WebCryptoAPI" \
    "file-system-access" \
    "workers" \
    "webaudio" \
    "serial" \
    "bluetooth" \
    "webgpu"

export REPO_URL="https://github.com/gpuweb/cts.git"
bash "$HELPER" "webgpu cts" "main" "$WEBGPU_CTS_REF" "$CTS_TARGET"
