#!/usr/bin/env bash
# Fetches Babel parser test fixtures from babel/babel
# https://github.com/babel/babel
#
# Version is pinned for reproducibility.
# Update BABEL_COMMIT when upgrading (also update babel.rs).

set -euo pipefail

# pinned version
BABEL_VERSION="7.26"
BABEL_COMMIT="b8ef443e0a3ee202264fb40edc1cbce8f2352aaa"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/babel"

# repo info
REPO_URL="https://github.com/babel/babel.git"
FIXTURES_PATH="packages/babel-parser/test/fixtures"

echo "fetching babel parser tests..."
echo "  version: $BABEL_VERSION"
echo "  commit:  ${BABEL_COMMIT:0:7}"
echo "  target:  $TARGET_DIR"

# clean existing
if [ -d "$TARGET_DIR" ]; then
    echo "  removing existing babel directory..."
    rm -rf "$TARGET_DIR"
fi

# sparse clone (only the fixtures directory)
echo "  cloning (sparse)..."
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

git init --quiet
git remote add origin "$REPO_URL"
git config core.sparseCheckout true
echo "$FIXTURES_PATH" >> .git/info/sparse-checkout
git fetch --quiet --depth 1 origin "$BABEL_COMMIT"
git checkout --quiet FETCH_HEAD

# move fixtures to target
mkdir -p "$TARGET_DIR"
mv "$FIXTURES_PATH"/* "$TARGET_DIR/"

# cleanup
cd "$SCRIPT_DIR"
rm -rf "$TEMP_DIR"

# write version file
echo "$BABEL_VERSION ($BABEL_COMMIT)" > "$TARGET_DIR/VERSION"

# count tests (directories with input.* files)
# note: flow tests are fetched but not run (we don't support Flow, only TypeScript)
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
