#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

CORPUS_ROOT="${1:-language/test/fixtures/formatter/conformance/staging/oxfmt}"
SNAPSHOT_OUT="${2:-/tmp/destack-formatter-corpus-snapshot}"

cargo run --release -p destack_formatter --example corpus -- snapshot --root "$CORPUS_ROOT" --out "$SNAPSHOT_OUT"
