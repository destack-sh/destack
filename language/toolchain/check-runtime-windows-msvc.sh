#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

if [ "${OS:-}" != "Windows_NT" ]; then
	echo "windows msvc runtime checks must run on a windows host"
	exit 1
fi

# prefer native windows toolchains inside git bash
windows_perl_directory="/c/Strawberry/perl/bin"
windows_nasm_directory="/c/Program Files/NASM"

if [ -d "${windows_perl_directory}" ]; then
	export PATH="${windows_perl_directory}:${PATH}"
	export PERL="${windows_perl_directory}/perl.exe"
fi

if [ -d "${windows_nasm_directory}" ]; then
	export PATH="${windows_nasm_directory}:${PATH}"
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

# confirm native perl and nasm resolve before openssl builds
command -v perl >/dev/null
command -v nasm >/dev/null

# check and test the full runtime crate on one real windows host
runtime_run_full_runtime_crate_lane "${cargo_bin}"

# run runtime smoke executable on host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 "${cargo_bin}" run -p tspp_runtime --bin runtime-smoke --quiet
