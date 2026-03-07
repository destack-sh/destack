#!/usr/bin/env bash
set -euo pipefail

if [ "${OS:-}" != "Windows_NT" ]; then
	echo "windows msvc runtime checks must run on a windows host"
	exit 1
fi

# use native rust executables on windows
cargo_bin="cargo"
if command -v cargo.exe >/dev/null 2>&1; then
	cargo_bin="cargo.exe"
fi

rustup_bin="rustup"
if command -v rustup.exe >/dev/null 2>&1; then
	rustup_bin="rustup.exe"
fi

# ensure target std is available before cargo check
if command -v "${rustup_bin}" >/dev/null 2>&1; then
	"${rustup_bin}" target add x86_64-pc-windows-msvc >/dev/null
fi

# check and lint runtime on windows host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 "${cargo_bin}" check -p destack_runtime
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 "${cargo_bin}" clippy -p destack_runtime --all-targets -- -D warnings

# run windows host adapter tests
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 "${cargo_bin}" test -p destack_runtime host::windows:: -- --nocapture

# run runtime smoke executable on host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 "${cargo_bin}" run -p destack_runtime --bin runtime-smoke --quiet
