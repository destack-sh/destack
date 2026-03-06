#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

if [ "$#" -lt 1 ]; then
	echo "usage: test-runtime-windows-gnu-crate.sh <crate> [test-filter] [-- <test-args...>]"
	exit 1
fi

target="$(runtime_windows_gnu_target)"
crate="$1"
test_filter="${DESTACK_WINDOWS_GNU_TEST_FILTER:-}"
test_timeout_seconds="${DESTACK_WINDOWS_GNU_TEST_TIMEOUT_SECONDS:-0}"
skip_audio_tests="${DESTACK_WINDOWS_GNU_SKIP_AUDIO_TESTS:-0}"
disable_wine_audio="${DESTACK_WINDOWS_GNU_DISABLE_WINE_AUDIO:-0}"
extra_test_arguments=()

shift

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

if [ "${skip_audio_tests}" = "1" ]; then
	extra_test_arguments+=("--skip" "platform::audio::tests::")
fi

# ensure target std is available before cargo test
runtime_ensure_rust_target "${target}"

# require host linker toolchain for windows gnu compilation
runtime_windows_gnu_require_build_toolchain

# run executable windows gnu tests only on linux and macos hosts
if ! runtime_windows_gnu_is_execution_host; then
	host_kernel="$(runtime_host_kernel)"
	echo "skipping windows gnu crate test execution on ${host_kernel}: run this lane on linux or macos"
	exit 0
fi

runtime_windows_gnu_require_wine

# build crate tests without running on host and collect executable artifacts
crate_artifact_output="$(mktemp)"
cleanup_crate_artifact_output() {
	rm -f "${crate_artifact_output}"
}
trap cleanup_crate_artifact_output EXIT

runtime_windows_gnu_run cargo test \
	-p "${crate}" \
	--target "${target}" \
	--no-run \
	--message-format=json-render-diagnostics >"${crate_artifact_output}"

# resolve all crate test executables produced by cargo
windows_test_executables=()
while IFS= read -r windows_test_executable; do
	if [ -n "${windows_test_executable}" ] && [ -f "${windows_test_executable}" ]; then
		windows_test_executables+=("${windows_test_executable}")
	fi
done < <(runtime_collect_cargo_test_executables "${crate_artifact_output}" "${crate}")

if [ "${#windows_test_executables[@]}" -eq 0 ]; then
	echo "missing windows test executables for crate: ${crate}"
	exit 1
fi

# run all crate test executables through wine
for windows_test_executable in "${windows_test_executables[@]}"; do
	crate_test_command=(wine "${windows_test_executable}")

	crate_test_environment=(env WINEDEBUG="${WINEDEBUG:--all}")
	if [ "${disable_wine_audio}" = "1" ]; then
		wine_audio_overrides="winecoreaudio.drv=d;winepulse.drv=d;winealsa.drv=d;xaudio2_7=d;winmm=d"
		if [ -n "${WINEDLLOVERRIDES:-}" ]; then
			wine_audio_overrides="${wine_audio_overrides};${WINEDLLOVERRIDES}"
		fi
		crate_test_environment+=("WINEDLLOVERRIDES=${wine_audio_overrides}")
	fi

	if [ -n "${test_filter}" ]; then
		crate_test_command+=("${test_filter}")
	fi
	if [ "${#extra_test_arguments[@]}" -gt 0 ]; then
		crate_test_command+=("${extra_test_arguments[@]}")
	fi

	runtime_run_with_optional_timeout \
		"${test_timeout_seconds}" \
		"windows gnu crate test command" \
		"${crate_test_environment[@]}" "${crate_test_command[@]}"
done
