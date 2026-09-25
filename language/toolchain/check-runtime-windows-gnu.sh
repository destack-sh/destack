#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment
runtime_ensure_rust_target x86_64-pc-windows-gnu

cc_wrapper="$(runtime_command_path x86_64-w64-mingw32-gcc)"
cxx_wrapper="$(runtime_command_path x86_64-w64-mingw32-g++)"
ar_wrapper="$(runtime_command_path x86_64-w64-mingw32-ar)"
ranlib_wrapper="$(runtime_command_path x86_64-w64-mingw32-ranlib)"
windres_wrapper="$(runtime_command_path x86_64-w64-mingw32-windres)"

if [ -z "${cc_wrapper}" ] || [ -z "${cxx_wrapper}" ] || [ -z "${ar_wrapper}" ] || [ -z "${ranlib_wrapper}" ] || [ -z "${windres_wrapper}" ]; then
	zig_path="$(runtime_command_path zig)"
	if [ -z "${zig_path}" ]; then
		echo "missing windows gnu toolchain: install x86_64-w64-mingw32 tools or zig"
		exit 1
	fi

	cc_wrapper="${script_directory}/x86_64-w64-mingw32-gcc"
	cxx_wrapper="${script_directory}/x86_64-w64-mingw32-g++"
	ar_wrapper="${script_directory}/ar-zig"
	ranlib_wrapper="${script_directory}/ranlib-zig"
	windres_wrapper="${script_directory}/x86_64-w64-mingw32-windres"
fi

run_windows_gnu_command() {
	env \
		CC_x86_64_pc_windows_gnu="${cc_wrapper}" \
		CXX_x86_64_pc_windows_gnu="${cxx_wrapper}" \
		AR_x86_64_pc_windows_gnu="${ar_wrapper}" \
		RANLIB_x86_64_pc_windows_gnu="${ranlib_wrapper}" \
		WINDRES_x86_64_pc_windows_gnu="${windres_wrapper}" \
		"CC_x86_64-pc-windows-gnu=${cc_wrapper}" \
		"CXX_x86_64-pc-windows-gnu=${cxx_wrapper}" \
		"AR_x86_64-pc-windows-gnu=${ar_wrapper}" \
		"RANLIB_x86_64-pc-windows-gnu=${ranlib_wrapper}" \
		"WINDRES_x86_64-pc-windows-gnu=${windres_wrapper}" \
		CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${cc_wrapper}" \
		"$@"
}

run_windows_gnu_command cargo check -p tspp_runtime --target x86_64-pc-windows-gnu
run_windows_gnu_command cargo clippy -p tspp_runtime --target x86_64-pc-windows-gnu --no-deps -- -D warnings
run_windows_gnu_command cargo test -p tspp_runtime --target x86_64-pc-windows-gnu --no-run
