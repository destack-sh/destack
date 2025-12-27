#!/usr/bin/env bash
# fetch bun types into builtin/lib/bun

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/bun"

BUN_TYPES_URL=${BUN_TYPES_URL:-"https://bun.sh/types/bun.d.ts"}

mkdir -p "$SRC_DIR"

echo "  - bun"
curl -fsSL "$BUN_TYPES_URL" > "$SRC_DIR/index.d.ds"
echo "done"
