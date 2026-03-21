#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/tests"
HELPER="$SCRIPT_DIR/../../fetch-origin.sh"

"$HELPER" "$SCRIPT_DIR"

PASS_COUNT=$(find "$TARGET_DIR/pass" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')
FAIL_COUNT=$(find "$TARGET_DIR/fail" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')
PASS_EXPLICIT_COUNT=$(find "$TARGET_DIR/pass-explicit" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')
EARLY_COUNT=$(find "$TARGET_DIR/early" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "done! test counts:"
echo "  pass:          $PASS_COUNT"
echo "  fail:          $FAIL_COUNT"
echo "  pass-explicit: $PASS_EXPLICIT_COUNT"
echo "  early:         $EARLY_COUNT"
echo "  total:         $((PASS_COUNT + FAIL_COUNT + PASS_EXPLICIT_COUNT + EARLY_COUNT))"
