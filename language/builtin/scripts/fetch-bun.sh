#!/usr/bin/env bash
# fetch bun types into builtin/lib/bun

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/bun"
source "$SCRIPT_DIR/fetch-util.sh"

BUN_TARGETS_DEFAULT=(
    "1.3:1.3.5"
)

BUN_TARGETS=("${BUN_TARGETS_DEFAULT[@]}")
if [[ -n "${BUN_TARGETS_OVERRIDE:-}" ]]; then
    IFS=',' read -r -a BUN_TARGETS <<< "$BUN_TARGETS_OVERRIDE"
fi

for target in "${BUN_TARGETS[@]}"; do
    IFS=':' read -r bun_lib_version bun_version <<< "$target"

    if [[ -z "$bun_lib_version" || -z "$bun_version" ]]; then
        fail "invalid bun target $target"
    fi

    BUN_TYPES_URL=${BUN_TYPES_URL:-"https://raw.githubusercontent.com/oven-sh/bun/bun-v$bun_version/packages/bun-types/bun.d.ts"}

    BUN_LIB_DIR="$SRC_DIR/v$bun_lib_version"
    mkdir -p "$BUN_LIB_DIR"

    echo "  - bun.v$bun_lib_version"
    fetch_url "$BUN_TYPES_URL" "$BUN_LIB_DIR/index.d.ds"
    echo "done"
done
