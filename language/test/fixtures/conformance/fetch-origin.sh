#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
	echo "usage: $0 <suite-dir>" >&2
	exit 1
fi

SUITE_DIR="$(cd "$1" && pwd)"
SUITE_JSON="$SUITE_DIR/suite.json"
TARGET_DIR="$SUITE_DIR/tests"
HELPER="$SUITE_DIR/../../fetch-fixtures.sh"

if [ ! -f "$SUITE_JSON" ]; then
	echo "error: missing suite.json in $SUITE_DIR" >&2
	exit 1
fi

SOURCE_COUNT="$(jq '[.fetch.entries[]? | select(.kind == "source")] | length' "$SUITE_JSON")"
if [ "$SOURCE_COUNT" -ne 1 ]; then
	echo "error: expected exactly one source fetch entry in $SUITE_JSON" >&2
	exit 1
fi

SOURCE_INDEX="$(jq -r '(.fetch.entries | to_entries | map(select(.value.kind == "source")) | .[0].key)' "$SUITE_JSON")"
ORIGIN_KIND="$(jq -r '.origin.kind' "$SUITE_JSON")"
SUITE_LABEL="$(jq -r ".fetch.entries[$SOURCE_INDEX].label // empty" "$SUITE_JSON")"
SUITE_VERSION="$(jq -r ".fetch.entries[$SOURCE_INDEX].version // \"unversioned\"" "$SUITE_JSON")"
SUITE_REF="$(jq -r '.origin.ref' "$SUITE_JSON")"
REPO_URL="$(jq -r '.origin.repo' "$SUITE_JSON")"
LAYOUT_MODE="$(jq -r ".fetch.entries[$SOURCE_INDEX].layout // \"preserve\"" "$SUITE_JSON")"

if [ "$ORIGIN_KIND" != "git" ]; then
	echo "error: unsupported origin kind '$ORIGIN_KIND' in $SUITE_JSON" >&2
	exit 1
fi

if [ -z "$SUITE_LABEL" ]; then
	SUITE_LABEL="$(basename "$SUITE_DIR")"
fi

IMPORT_PATHS=()
while IFS= read -r path; do
	IMPORT_PATHS+=("$path")
done < <(jq -r ".fetch.entries[$SOURCE_INDEX].paths[]?" "$SUITE_JSON")

INCLUDE_PATTERNS="$(jq -r ".fetch.entries[$SOURCE_INDEX].include[]?" "$SUITE_JSON")"
EXCLUDE_PATTERNS="$(jq -r ".fetch.entries[$SOURCE_INDEX].exclude[]?" "$SUITE_JSON")"

export REPO_URL
export LAYOUT_MODE
export INCLUDE_PATTERNS
export EXCLUDE_PATTERNS

"$HELPER" "$SUITE_LABEL" "$SUITE_VERSION" "$SUITE_REF" "$TARGET_DIR" "${IMPORT_PATHS[@]}"
