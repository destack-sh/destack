#!/usr/bin/env bash
set -euo pipefail

target="wasm32-wasip1"

# ensure target std is available before cargo check
if command -v rustup >/dev/null 2>&1; then
	rustup target add "${target}" >/dev/null
fi

# check runtime for wasip1 target
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo check -p destack_runtime --target "${target}"
