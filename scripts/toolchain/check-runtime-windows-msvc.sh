#!/usr/bin/env bash
set -euo pipefail

if [ "${OS:-}" != "Windows_NT" ]; then
    echo "windows msvc runtime checks must run on a windows host"
    exit 1
fi

# check and lint runtime on windows host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo check -p destack_runtime
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo clippy -p destack_runtime --all-targets -- -D warnings

# run windows host adapter tests
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo test -p destack_runtime runtime::host::windows:: -- --nocapture

# run runtime smoke executable on host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo run -p destack_runtime --bin runtime-smoke --quiet
