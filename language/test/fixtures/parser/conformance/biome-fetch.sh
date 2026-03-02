#!/usr/bin/env bash
# Fetches Biome parser test fixtures from biomejs/biome
# https://github.com/biomejs/biome
#
# Version is pinned for reproducibility.
# Update BIOME_COMMIT when upgrading (also update biome.rs).

set -euo pipefail

# pinned version
BIOME_VERSION="1.x"
BIOME_COMMIT="9f1b3b06586401b39e0aa886bf7c8484fd2a6ded"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/biome"

# repo info
REPO_URL="https://github.com/biomejs/biome.git"
FIXTURES_PATH="crates/biome_js_parser/tests/js_test_suite"

echo "fetching biome parser tests..."
echo "  version: $BIOME_VERSION"
echo "  commit:  ${BIOME_COMMIT:0:7}"
echo "  target:  $TARGET_DIR"

# clean existing
if [ -d "$TARGET_DIR" ]; then
    echo "  removing existing biome directory..."
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
git fetch --quiet --depth 1 origin "$BIOME_COMMIT"
git checkout --quiet FETCH_HEAD

# move fixtures to target
mkdir -p "$TARGET_DIR"
mv "$FIXTURES_PATH"/* "$TARGET_DIR/"

# cleanup
cd "$SCRIPT_DIR"
rm -rf "$TEMP_DIR"

# write version file
echo "$BIOME_VERSION ($BIOME_COMMIT)" > "$TARGET_DIR/VERSION"

# count tests
JS_COUNT=$(find "$TARGET_DIR" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')
TS_COUNT=$(find "$TARGET_DIR" -name "*.ts" ! -name "*.d.ts" 2>/dev/null | wc -l | tr -d ' ')
TSX_COUNT=$(find "$TARGET_DIR" -name "*.tsx" 2>/dev/null | wc -l | tr -d ' ')
JSX_COUNT=$(find "$TARGET_DIR" -name "*.jsx" 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "done! test counts:"
echo "  js:         $JS_COUNT"
echo "  typescript: $TS_COUNT"
echo "  tsx:        $TSX_COUNT"
echo "  jsx:        $JSX_COUNT"
echo "  total:      $((JS_COUNT + TS_COUNT + TSX_COUNT + JSX_COUNT))"
