#!/usr/bin/env bash
# fetch bun types into builtin/lib/bun

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/bun"

BUN_VERSION=${BUN_VERSION:-"1.1.0"}
BUN_LIB_VERSION=${BUN_LIB_VERSION:-"1.1"}
BUN_TYPES_URL=${BUN_TYPES_URL:-"https://raw.githubusercontent.com/oven-sh/bun/bun-v$BUN_VERSION/packages/bun-types/bun.d.ts"}

BUN_LIB_DIR="$SRC_DIR/v$BUN_LIB_VERSION"
mkdir -p "$BUN_LIB_DIR"

echo "  - bun.v$BUN_LIB_VERSION"
curl -fsSL "$BUN_TYPES_URL" > "$BUN_LIB_DIR/index.d.ds"
echo "done"
