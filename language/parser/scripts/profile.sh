#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
export LANG=C

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

SOURCE_INPUT="${1:-language/library/src/collections/array.tspp}"
DURATION="${2:-10}"
MODE="${3:-trivia}"
OUTPUT_INPUT="${4:-}"

if [[ "$SOURCE_INPUT" = /* ]]; then
  SOURCE_FILE="$SOURCE_INPUT"
else
  SOURCE_FILE="$ROOT_DIR/$SOURCE_INPUT"
fi

if [[ ! -f "$SOURCE_FILE" ]]; then
  echo "missing source file: $SOURCE_FILE"
  exit 2
fi

case "$MODE" in
  trivia | full)
    TRIVIA_MODE="full"
    OUTPUT_MODE="full"
    ;;
  doc | docs | documentation)
    TRIVIA_MODE="documentation"
    OUTPUT_MODE="documentation"
    ;;
  no-trivia | bare | ignore)
    TRIVIA_MODE="ignore"
    OUTPUT_MODE="ignore"
    ;;
  *)
    echo "unknown mode: $MODE"
    echo "expected one of: full, documentation, ignore"
    exit 2
    ;;
esac

if [[ -n "$OUTPUT_INPUT" ]]; then
  OUTPUT_FILE="$OUTPUT_INPUT"
else
  SOURCE_NAME="$(basename "$SOURCE_FILE" | tr '/.' '__')"
  OUTPUT_FILE="$ROOT_DIR/language/parser/target/flamegraphs/${SOURCE_NAME}.${OUTPUT_MODE}.svg"
fi

TSPP_PARSE_SECONDS="$DURATION" \
TSPP_PARSE_TRIVIA="$TRIVIA_MODE" \
TSPP_PARSE_OUTPUT="$OUTPUT_FILE" \
  cargo run --profile bench -p tspp_parser --example parse -- "$SOURCE_FILE"

if [[ ! -f "$OUTPUT_FILE" ]]; then
  echo "missing flamegraph: $OUTPUT_FILE"
  exit 1
fi

"$ROOT_DIR/language/parser/scripts/flamegraph.sh" "$OUTPUT_FILE"
