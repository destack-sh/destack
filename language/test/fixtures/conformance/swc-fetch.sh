#!/usr/bin/env bash
# Fetches SWC parser test fixtures from swc-project/swc
# https://github.com/swc-project/swc
#
# Version is pinned for reproducibility.
# Update SWC_COMMIT when upgrading (also update swc.rs).

set -euo pipefail

# pinned version
SWC_VERSION="1.x"
SWC_COMMIT="5b9d77c1c89ade5772c6feee429386faf3b93a39"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/swc"

# repo info
REPO_URL="https://github.com/swc-project/swc.git"
FIXTURES_PATH="crates/swc_ecma_parser/tests"

echo "fetching swc parser tests..."
echo "  version: $SWC_VERSION"
echo "  commit:  ${SWC_COMMIT:0:7}"
echo "  target:  $TARGET_DIR"

# clean existing
if [ -d "$TARGET_DIR" ]; then
    echo "  removing existing swc directory..."
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
git fetch --quiet --depth 1 origin "$SWC_COMMIT"
git checkout --quiet FETCH_HEAD

# move fixtures to target
mkdir -p "$TARGET_DIR"
mv "$FIXTURES_PATH"/* "$TARGET_DIR/"

# cleanup
cd "$SCRIPT_DIR"
rm -rf "$TEMP_DIR"

# write version file
echo "$SWC_VERSION ($SWC_COMMIT)" > "$TARGET_DIR/VERSION"

# count tests
TS_COUNT=$(find "$TARGET_DIR/typescript" -name "*.ts" ! -name "*.d.ts" 2>/dev/null | wc -l | tr -d ' ')
TSX_COUNT=$(find "$TARGET_DIR/typescript" -name "*.tsx" 2>/dev/null | wc -l | tr -d ' ')
JSX_COUNT=$(find "$TARGET_DIR/jsx" -name "*.js" -o -name "*.jsx" 2>/dev/null | wc -l | tr -d ' ')
JS_COUNT=$(find "$TARGET_DIR/js" -name "*.js" 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "done! test counts:"
echo "  typescript: $TS_COUNT"
echo "  tsx:        $TSX_COUNT"
echo "  jsx:        $JSX_COUNT"
echo "  js:         $JS_COUNT"
echo "  total:      $((TS_COUNT + TSX_COUNT + JSX_COUNT + JS_COUNT))"
