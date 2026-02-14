#!/usr/bin/env bash
set -euo pipefail

SUITE_NAME="prettier"
SUITE_VERSION="3.x"
SUITE_COMMIT="${PRETTIER_COMMIT:-456d14dbc2639d5a6b3eb2d8836b30f63597e1d6}"
REPO_URL="https://github.com/prettier/prettier.git"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/staging/$SUITE_NAME"
TMP_DIR="$(mktemp -d)"

PATHS=(
    "tests/format"
)

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "fetching $SUITE_NAME formatter fixtures..."
echo "  version: $SUITE_VERSION"
echo "  commit:  ${SUITE_COMMIT:0:7}"
echo "  target:  $TARGET_DIR"

if [ -d "$TARGET_DIR" ]; then
    rm -rf "$TARGET_DIR"
fi
mkdir -p "$TARGET_DIR"

cd "$TMP_DIR"
git init --quiet
git remote add origin "$REPO_URL"
git config core.sparseCheckout true
for path in "${PATHS[@]}"; do
    echo "$path" >> .git/info/sparse-checkout
done
git fetch --quiet --depth 1 origin "$SUITE_COMMIT"
git checkout --quiet FETCH_HEAD

FOUND=0
for path in "${PATHS[@]}"; do
    if [ -e "$path" ]; then
        mkdir -p "$TARGET_DIR/$(dirname "$path")"
        cp -R "$path" "$TARGET_DIR/$path"
        FOUND=1
    fi
done

if [ "$FOUND" -eq 0 ]; then
    echo "error: no expected fixture paths were found in $SUITE_NAME" >&2
    exit 1
fi

printf "%s (%s)\n" "$SUITE_VERSION" "$SUITE_COMMIT" > "$TARGET_DIR/VERSION"

JS_COUNT=$(find "$TARGET_DIR" -type f -name "*.js" 2>/dev/null | wc -l | tr -d ' ')
JSX_COUNT=$(find "$TARGET_DIR" -type f -name "*.jsx" 2>/dev/null | wc -l | tr -d ' ')
TS_COUNT=$(find "$TARGET_DIR" -type f -name "*.ts" ! -name "*.d.ts" 2>/dev/null | wc -l | tr -d ' ')
TSX_COUNT=$(find "$TARGET_DIR" -type f -name "*.tsx" 2>/dev/null | wc -l | tr -d ' ')
DTS_COUNT=$(find "$TARGET_DIR" -type f -name "*.d.ts" 2>/dev/null | wc -l | tr -d ' ')

echo ""
echo "done! source file counts:"
echo "  js:   $JS_COUNT"
echo "  jsx:  $JSX_COUNT"
echo "  ts:   $TS_COUNT"
echo "  tsx:  $TSX_COUNT"
echo "  d.ts: $DTS_COUNT"
echo "  total: $((JS_COUNT + JSX_COUNT + TS_COUNT + TSX_COUNT + DTS_COUNT))"
