#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
export LANG=C

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

SOURCE_FILE_INPUT="${1:-language/test/fixtures/parser/oxc/typescript.js}"
PHASE="${2:-parse}"
PROFILE_SECONDS="${3:-10}"

# resolve source path so criterion benchmark cwd does not matter
if [[ "$SOURCE_FILE_INPUT" = /* ]]; then
  SOURCE_FILE="$SOURCE_FILE_INPUT"
else
  SOURCE_FILE="$ROOT_DIR/$SOURCE_FILE_INPUT"
fi

if [[ ! -f "$SOURCE_FILE" ]]; then
  echo "missing source file: $SOURCE_FILE"
  exit 2
fi

# map phase to criterion bench path
case "$PHASE" in
  parse)
    BENCH_PATH="parse/single-thread"
    ;;
  parse-no-drop)
    BENCH_PATH="parse/no-drop"
    ;;
  main)
    BENCH_PATH="main/single-thread"
    ;;
  *)
    echo "unknown phase: $PHASE"
    echo "expected one of: parse, parse-no-drop, main"
    exit 2
    ;;
esac

# run criterion profiling for the selected file and phase
DESTACK_PARSE_FILE="$SOURCE_FILE" \
  cargo bench -p destack_parser --bench destack_parse -- "$BENCH_PATH" --profile-time "$PROFILE_SECONDS"

FLAMEGRAPH_PATH="target/criterion/destack_parser_single/${BENCH_PATH}/profile/flamegraph.svg"

if [[ ! -f "$FLAMEGRAPH_PATH" ]]; then
  echo "missing flamegraph at: $FLAMEGRAPH_PATH"
  exit 1
fi

echo "flamegraph: $FLAMEGRAPH_PATH"

# summarize hotspots from the generated flamegraph
"$ROOT_DIR/language/parser/scripts/summarize_flamegraph.sh" "$FLAMEGRAPH_PATH"
