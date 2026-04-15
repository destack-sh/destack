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
INCLUDE_PATTERNS="${INCLUDE_PATTERNS:-}"
EXCLUDE_PATTERNS="${EXCLUDE_PATTERNS:-}"
TMP_DIR="$(mktemp -d)"

cleanup() {
	rm -rf "$TMP_DIR"
}
trap cleanup EXIT

copy_tree() {
	local source_dir="$1"
	local destination_dir="$2"

	python3 - "$source_dir" "$destination_dir" <<'PY'
from __future__ import annotations

import os
import shutil
import sys
from fnmatch import fnmatchcase
from pathlib import Path


def load_patterns(name: str) -> list[str]:
    value = os.environ.get(name, "")
    return [pattern for pattern in value.splitlines() if pattern]


def matches(path: str, patterns: list[str]) -> bool:
    for pattern in patterns:
        if fnmatchcase(path, pattern):
            return True
        if pattern.startswith("**/") and fnmatchcase(path, pattern[3:]):
            return True
        if "/**/" in pattern and fnmatchcase(path, pattern.replace("/**/", "/")):
            return True
    return False


source_dir = Path(sys.argv[1])
destination_dir = Path(sys.argv[2])
include_patterns = load_patterns("INCLUDE_PATTERNS")
exclude_patterns = load_patterns("EXCLUDE_PATTERNS")

for source_path in sorted(source_dir.rglob("*")):
    if not source_path.is_file():
        continue

    relative_path = source_path.relative_to(source_dir).as_posix()

    if include_patterns and not matches(relative_path, include_patterns):
        continue

    if exclude_patterns and matches(relative_path, exclude_patterns):
        continue

    destination_path = destination_dir / relative_path
    destination_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source_path, destination_path)
PY
}

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
		echo "$path" >>.git/info/sparse-checkout
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

		copy_tree "$path" "$TARGET_DIR"
	else
		found=0

		for path in "$@"; do
			if [ -e "$path" ]; then
				if [ -d "$path" ]; then
					copy_tree "$path" "$TARGET_DIR/$path"
				else
					mkdir -p "$TARGET_DIR/$(dirname "$path")"
					cp -R "$path" "$TARGET_DIR/$path"
				fi
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
printf "%s (%s)\n" "$SUITE_VERSION" "$SUITE_COMMIT" >"$TARGET_DIR/VERSION"
