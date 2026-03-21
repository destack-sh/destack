#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/tests"
HELPER="$SCRIPT_DIR/../../fetch-origin.sh"

"$HELPER" "$SCRIPT_DIR"

TS_COUNT=$(find "$TARGET_DIR/typescript" -name "input.ts" 2>/dev/null | wc -l | tr -d ' ')
TSX_COUNT=$(find "$TARGET_DIR/typescript" -name "input.tsx" 2>/dev/null | wc -l | tr -d ' ')
JSX_COUNT=$(find "$TARGET_DIR/jsx" -name "input.js" 2>/dev/null | wc -l | tr -d ' ')
JS_COUNT=$(find "$TARGET_DIR" -path "*/es*" -name "input.js" 2>/dev/null | wc -l | tr -d ' ')
FLOW_COUNT=$(find "$TARGET_DIR/flow" -name "input.js" 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "done! test counts:"
echo "  typescript: $TS_COUNT"
echo "  tsx:        $TSX_COUNT"
echo "  jsx:        $JSX_COUNT"
echo "  es*:        $JS_COUNT"
echo "  flow:       $FLOW_COUNT (skipped, not run)"
echo "  total:      $((TS_COUNT + TSX_COUNT + JSX_COUNT + JS_COUNT)) (excluding flow)"
