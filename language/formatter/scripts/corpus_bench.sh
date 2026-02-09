#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

CORPUS_ROOT="${1:-language/test/fixtures/ecosystem}"
RUNS="${2:-5}"

cargo run --release -p destack_formatter --example bench_stats -- --root "$CORPUS_ROOT" --runs "$RUNS"
