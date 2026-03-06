#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/versions.sh
source "${script_directory}/versions.sh"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

has_error="0"
host_kernel="$(runtime_host_kernel)"
host_arch="$(uname -m)"

print_ok() {
	message="$1"
	printf '[ok] %s\n' "${message}"
}

print_warn() {
	message="$1"
	printf '[warn] %s\n' "${message}"
}

print_error() {
	message="$1"
	printf '[error] %s\n' "${message}"
	has_error="1"
}

check_command() {
	command_name="$1"
	required="$2"
	description="$3"

	command_path="$(runtime_command_path "${command_name}")"
	if [ -n "${command_path}" ]; then
		print_ok "${description}: ${command_path}"
		return 0
	fi

	if [ "${required}" = "required" ]; then
		print_error "${description}: missing command '${command_name}'"
	else
		print_warn "${description}: missing optional command '${command_name}'"
	fi
}

check_rust_target() {
	target="$1"
	required="$2"

	if runtime_rust_target_installed "${target}"; then
		print_ok "rust target installed: ${target}"
		return 0
	fi

	if [ "${required}" = "required" ]; then
		print_error "rust target missing: ${target}"
	else
		print_warn "rust target missing: ${target}"
	fi
}

printf 'runtime toolchain doctor\n'
printf 'host: %s (%s)\n' "${host_kernel}" "${host_arch}"

check_command rustup required "rustup"
check_command cargo required "cargo"
check_command just required "just"

# ensure the active rustup default toolchain matches the pinned ci toolchain
if [ -n "$(runtime_command_path rustup)" ]; then
	default_toolchain="$(rustup default 2>/dev/null | awk '{print $1}' || true)"
	if [ "${default_toolchain}" = "${DESTACK_RUST_TOOLCHAIN}" ]; then
		print_ok "rustup default toolchain: ${default_toolchain}"
	else
		print_warn "rustup default toolchain is ${default_toolchain}, expected ${DESTACK_RUST_TOOLCHAIN}"
	fi
fi

# zig is required by runtime cross lanes:
# linux gnu targets use zig linker wrappers on all supported hosts
check_command zig required "zig"

# wine is required when windows gnu tests execute through wine
if runtime_windows_gnu_is_execution_host; then
	check_command wine required "wine"
	check_command python3 required "python3"
else
	check_command wine optional "wine"
	check_command python3 optional "python3"
fi

# mingw-w64 compiler is required on macos for windows gnu linker wiring
if [ "${host_kernel}" = "Darwin" ]; then
	check_command x86_64-w64-mingw32-gcc required "mingw-w64 compiler"
else
	check_command x86_64-w64-mingw32-gcc optional "mingw-w64 compiler"
fi

# host sdk checks
if [ "${host_kernel}" = "Darwin" ]; then
	check_command xcrun required "xcode sdk tools"
fi

# rust targets used by runtime lanes
check_rust_target wasm32-wasip1 required
check_rust_target x86_64-pc-windows-gnu required
check_rust_target aarch64-linux-android required

# ios checks only apply on macos hosts
if [ "${host_kernel}" = "Darwin" ]; then
	check_rust_target aarch64-apple-ios required
else
	check_rust_target aarch64-apple-ios optional
fi

# linux gnu cross checks are first class on linux hosts
if [ "${host_kernel}" = "Linux" ]; then
	check_rust_target x86_64-unknown-linux-gnu required
	check_rust_target aarch64-unknown-linux-gnu required
	check_command weston required "weston wayland compositor"
else
	check_rust_target x86_64-unknown-linux-gnu optional
	check_rust_target aarch64-unknown-linux-gnu optional
	check_command weston optional "weston wayland compositor"
fi

# android ndk resolution
if ndk_root="$("${script_directory}"/resolve-android-ndk-root.sh 2>/dev/null)"; then
	print_ok "android ndk root: ${ndk_root}"
else
	print_error "android ndk root: not found, run just language/install-toolchain"
fi

# dbus host pkg-config is not required:
# keyring linux secret-service path is built vendored for cross determinism
print_ok "dbus host pkg-config is not required for runtime cross lanes"

if [ "${has_error}" = "1" ]; then
	printf 'runtime toolchain doctor: failed\n'
	exit 1
fi

printf 'runtime toolchain doctor: ok\n'
