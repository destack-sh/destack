#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/tests"
HELPER="$SCRIPT_DIR/../../fetch-origin.sh"

"$HELPER" "$SCRIPT_DIR"

TS_COUNT=$(find "$TARGET_DIR/typescript" -name "*.ts" ! -name "*.d.ts" 2>/dev/null | wc -l | tr -d ' ')
TSX_COUNT=$(find "$TARGET_DIR/typescript" -name "*.tsx" 2>/dev/null | wc -l | tr -d ' ')
JSX_COUNT=$(find "$TARGET_DIR/jsx" \( -name "*.js" -o -name "*.jsx" \) 2>/dev/null | wc -l | tr -d ' ')
JS_COUNT=$(find "$TARGET_DIR/js" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "done! test counts:"
echo "  typescript: $TS_COUNT"
echo "  tsx:        $TSX_COUNT"
echo "  jsx:        $JSX_COUNT"
echo "  js:         $JS_COUNT"
echo "  total:      $((TS_COUNT + TSX_COUNT + JSX_COUNT + JS_COUNT))"
