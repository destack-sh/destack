#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

target="$(runtime_windows_gnu_target)"
profile="debug"

for argument in "$@"; do
    if [ "${argument}" = "--release" ]; then
        profile="release"
        continue
    fi

    echo "unknown argument: ${argument}"
    echo "usage: runtime-windows-gnu-smoke.sh [--release]"
    exit 1
done

# ensure target std is available before cargo build
runtime_ensure_rust_target "${target}"

# require host linker toolchain for windows gnu compilation
runtime_windows_gnu_require_build_toolchain

# run executable windows gnu smoke checks only on linux and macos hosts
if ! runtime_windows_gnu_is_execution_host; then
    host_kernel="$(runtime_host_kernel)"
    echo "skipping windows gnu runtime smoke execution on ${host_kernel}: run this lane on linux or macos"
    exit 0
fi

runtime_windows_gnu_require_wine

# build runtime smoke executable in the selected profile
if [ "${profile}" = "release" ]; then
    runtime_windows_gnu_run cargo build --release -p destack_runtime --target "${target}" --bin runtime-smoke
else
    runtime_windows_gnu_run cargo build -p destack_runtime --target "${target}" --bin runtime-smoke
fi

# resolve target output path for this workspace invocation
exe_directory="$(runtime_windows_gnu_exe_directory "${target}" "${profile}")"
runtime_smoke_executable="${exe_directory}/runtime-smoke.exe"

if [ ! -f "${runtime_smoke_executable}" ]; then
    echo "missing windows gnu runtime smoke executable"
    exit 1
fi

# run runtime smoke executable through wine
WINEDEBUG="${WINEDEBUG:--all}" wine "${runtime_smoke_executable}"
