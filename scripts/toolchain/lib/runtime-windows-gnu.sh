#!/usr/bin/env bash

runtime_windows_gnu_target() {
	printf '%s\n' x86_64-pc-windows-gnu
}

runtime_windows_gnu_apply_environment() {
	runtime_set_standard_environment

	host_kernel="$(runtime_host_kernel)"

	if [ "${host_kernel}" = "Darwin" ]; then
		runtime_require_command x86_64-w64-mingw32-gcc \
			"missing x86_64-w64-mingw32-gcc: install mingw-w64 to build windows gnu targets" || return 1

		export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
		export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
	fi
}

runtime_windows_gnu_run() {
	runtime_windows_gnu_apply_environment || return 1
	"$@"
}

runtime_windows_gnu_require_build_toolchain() {
	host_kernel="$(runtime_host_kernel)"

	if [ "${host_kernel}" = "Linux" ]; then
		runtime_require_command zig "missing zig: install zig to build windows gnu targets" || return 1
		return 0
	fi

	if [ "${host_kernel}" = "Darwin" ]; then
		runtime_require_command x86_64-w64-mingw32-gcc \
			"missing x86_64-w64-mingw32-gcc: install mingw-w64 to build windows gnu targets" || return 1
	fi
}

runtime_windows_gnu_is_execution_host() {
	host_kernel="$(runtime_host_kernel)"

	if [ "${host_kernel}" = "Linux" ] || [ "${host_kernel}" = "Darwin" ]; then
		return 0
	fi

	return 1
}

runtime_windows_gnu_require_wine() {
	runtime_require_or_auto_install_linux_command \
		wine \
		wine64 \
		"missing wine: install wine to run windows gnu executables" \
		"just language/check-runtime-windows-gnu"
}

runtime_windows_gnu_exe_directory() {
	target="$1"
	profile="$2"

	primary_directory="../target/${target}/${profile}"
	fallback_directory="target/${target}/${profile}"

	if [ -d "${primary_directory}" ]; then
		printf '%s\n' "${primary_directory}"
		return 0
	fi

	printf '%s\n' "${fallback_directory}"
}
