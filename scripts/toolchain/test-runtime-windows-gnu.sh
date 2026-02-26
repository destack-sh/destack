#!/usr/bin/env bash
set -euo pipefail

target="x86_64-pc-windows-gnu"
primary_exe_directory="../target/${target}/debug"
fallback_exe_directory="target/${target}/debug"

# run executable windows gnu tests only on linux hosts
host_kernel="$(uname -s)"
if [ "${host_kernel}" != "Linux" ]; then
    echo "skipping windows gnu runtime test execution on ${host_kernel}: run this lane on linux or ci"
    exit 0
fi

if ! command -v zig >/dev/null 2>&1; then
    echo "missing zig: install zig to build windows gnu targets"
    exit 1
fi

if ! command -v wine >/dev/null 2>&1; then
    echo "missing wine: install wine to run windows gnu runtime tests"
    exit 1
fi

# build runtime tests without running on host
LC_ALL=C LANG=C CARGO_INCREMENTAL=0 cargo test -p destack_runtime --target "${target}" --no-run

# choose target directory for current workspace invocation
exe_directory="${primary_exe_directory}"
if [ ! -d "${exe_directory}" ]; then
    exe_directory="${fallback_exe_directory}"
fi
deps_directory="${exe_directory}/deps"

# locate newest runtime test executable and run it through wine
runtime_test_executable="$(ls -t "${deps_directory}"/destack_runtime-*.exe 2>/dev/null | head -n1 || true)"
if [ -z "${runtime_test_executable}" ] || [ ! -f "${runtime_test_executable}" ]; then
    echo "missing windows runtime test executable"
    exit 1
fi
WINEDEBUG="${WINEDEBUG:--all}" wine "${runtime_test_executable}"
