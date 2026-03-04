#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

target="$(runtime_windows_gnu_target)"
test_filter="${DESTACK_WINDOWS_GNU_TEST_FILTER:-}"
test_timeout_seconds="${DESTACK_WINDOWS_GNU_TEST_TIMEOUT_SECONDS:-0}"
extra_test_arguments=()

if [ "$#" -gt 0 ]; then
	if [ "$1" != "--" ]; then
		test_filter="$1"
		shift
	fi
fi

if [ "$#" -gt 0 ]; then
	if [ "$1" = "--" ]; then
		shift
	fi
	extra_test_arguments=("$@")
fi

# ensure target std is available before cargo test
runtime_ensure_rust_target "${target}"

# require host linker toolchain for windows gnu compilation
runtime_windows_gnu_require_build_toolchain

# run executable windows gnu tests only on linux and macos hosts
if ! runtime_windows_gnu_is_execution_host; then
	host_kernel="$(runtime_host_kernel)"
	echo "skipping windows gnu runtime test execution on ${host_kernel}: run this lane on linux or macos"
	exit 0
fi

runtime_windows_gnu_require_wine

# build runtime tests without running on host and collect executable artifacts
runtime_artifact_output="$(mktemp)"
cleanup_runtime_artifact_output() {
	rm -f "${runtime_artifact_output}"
}
trap cleanup_runtime_artifact_output EXIT

runtime_windows_gnu_run cargo test \
	-p destack_runtime \
	--target "${target}" \
	--no-run \
	--message-format=json-render-diagnostics >"${runtime_artifact_output}"

# resolve all runtime test executables produced by cargo
runtime_test_executables=()
while IFS= read -r runtime_test_executable; do
	if [ -n "${runtime_test_executable}" ] && [ -f "${runtime_test_executable}" ]; then
		runtime_test_executables+=("${runtime_test_executable}")
	fi
done < <(runtime_collect_cargo_test_executables "${runtime_artifact_output}" "destack_runtime")

if [ "${#runtime_test_executables[@]}" -eq 0 ]; then
	echo "missing windows runtime test executables"
	exit 1
fi

# run all runtime test executables through wine
for runtime_test_executable in "${runtime_test_executables[@]}"; do
	runtime_test_command=(wine "${runtime_test_executable}")
	if [ -n "${test_filter}" ]; then
		runtime_test_command+=("${test_filter}")
	fi
	if [ "${#extra_test_arguments[@]}" -gt 0 ]; then
		runtime_test_command+=("--" "${extra_test_arguments[@]}")
	fi

	runtime_run_with_optional_timeout \
		"${test_timeout_seconds}" \
		"windows gnu runtime test command" \
		env WINEDEBUG="${WINEDEBUG:--all}" "${runtime_test_command[@]}"
done
