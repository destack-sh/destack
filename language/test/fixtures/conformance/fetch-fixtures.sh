#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ]; then
    echo "usage: $0 <suite-name> <suite-version> <suite-commit> <target-dir> [path ...]" >&2
    exit 1
fi

SUITE_NAME="$1"
SUITE_VERSION="$2"
SUITE_COMMIT="$3"
TARGET_DIR="$4"
shift 4

REPO_URL="${REPO_URL:?missing REPO_URL}"
LAYOUT_MODE="${LAYOUT_MODE:-preserve}"
TMP_DIR="$(mktemp -d)"

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "fetching $SUITE_NAME fixtures..."
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

if [ "$#" -gt 0 ]; then
    git config core.sparseCheckout true
    for path in "$@"; do
        echo "$path" >> .git/info/sparse-checkout
    done
fi

git fetch --quiet --depth 1 origin "$SUITE_COMMIT"
git checkout --quiet FETCH_HEAD

if [ "$#" -gt 0 ]; then
    if [ "$LAYOUT_MODE" = "strip" ]; then
        if [ "$#" -ne 1 ]; then
            echo "error: strip layout mode expects exactly one imported path" >&2
            exit 1
        fi

        path="$1"
        if [ ! -d "$path" ]; then
            echo "error: expected imported directory '$path' in $SUITE_NAME" >&2
            exit 1
        fi

        cp -R "$path"/. "$TARGET_DIR/"
    else
        found=0

        for path in "$@"; do
            if [ -e "$path" ]; then
                mkdir -p "$TARGET_DIR/$(dirname "$path")"
                cp -R "$path" "$TARGET_DIR/$path"
                found=1
            fi
        done

        if [ "$found" -eq 0 ]; then
            echo "error: no expected fixture paths were found in $SUITE_NAME" >&2
            exit 1
        fi
    fi
else
    cp -R . "$TARGET_DIR/"
fi

rm -rf "$TARGET_DIR/.git"
printf "%s (%s)\n" "$SUITE_VERSION" "$SUITE_COMMIT" > "$TARGET_DIR/VERSION"
