#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

target="$(runtime_windows_gnu_target)"
runtime_test_script="${script_directory}/test-runtime-windows-gnu.sh"
runtime_smoke_script="${script_directory}/smoke-runtime-windows-gnu.sh"

# ensure target std is available before cargo check
runtime_ensure_rust_target "${target}"

# require host linker toolchain for windows gnu compilation
runtime_windows_gnu_require_build_toolchain

# check and lint windows gnu runtime target
runtime_windows_gnu_run cargo check -p destack_runtime --target "${target}"
runtime_windows_gnu_run cargo clippy -p destack_runtime --target "${target}" --no-deps -- -D warnings

# run executable windows gnu checks only on linux and macos hosts
if ! runtime_windows_gnu_is_execution_host; then
	host_kernel="$(runtime_host_kernel)"
	echo "skipping windows gnu executable checks on ${host_kernel}: run this lane on linux or macos"
	exit 0
fi

runtime_windows_gnu_require_wine

# build and run runtime tests through the shared test script
"${runtime_test_script}" "$@"

# build and run runtime smoke executable through the shared smoke script
"${runtime_smoke_script}"
