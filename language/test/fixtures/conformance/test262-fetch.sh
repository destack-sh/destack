#!/usr/bin/env bash
# fetches test262 parser tests from tc39/test262-parser-tests
# https://github.com/tc39/test262-parser-tests
#
# version is pinned to ensure reproducible test results
# update TEST262_COMMIT when upgrading (also update test262.rs)

set -euo pipefail

# pinned version
TEST262_VERSION="0.1.0"
TEST262_COMMIT="521b4ab99d50d1b88522deab7dd4ac37a56f9af0"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/test262"

# repo info
REPO_URL="https://github.com/tc39/test262-parser-tests.git"

echo "fetching test262 parser tests..."
echo "  version: $TEST262_VERSION"
echo "  commit:  ${TEST262_COMMIT:0:7}"
echo "  target:  $TARGET_DIR"

# clean existing
if [ -d "$TARGET_DIR" ]; then
    echo "  removing existing test262 directory..."
    rm -rf "$TARGET_DIR"
fi

# clone at specific commit
echo "  cloning..."
git clone --quiet "$REPO_URL" "$TARGET_DIR"
cd "$TARGET_DIR"
git checkout --quiet "$TEST262_COMMIT"
cd ..

# remove .git to save space and avoid nested repo issues
rm -rf "$TARGET_DIR/.git"

# write version file for tracking
echo "$TEST262_VERSION ($TEST262_COMMIT)" > "$TARGET_DIR/VERSION"

# count tests
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
