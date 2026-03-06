#!/usr/bin/env bash
set -euo pipefail

if [ "${OS:-}" != "Windows_NT" ]; then
	echo "windows msvc runtime checks must run on a windows host"
	exit 1
fi

# ensure target std is available before cargo check
if command -v rustup >/dev/null 2>&1; then
	rustup target add x86_64-pc-windows-msvc >/dev/null
fi

# check and lint runtime on windows host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo check -p destack_runtime
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo clippy -p destack_runtime --all-targets -- -D warnings

# run windows host adapter tests
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo test -p destack_runtime host::windows:: -- --nocapture

# run runtime smoke executable on host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo run -p destack_runtime --bin runtime-smoke --quiet
