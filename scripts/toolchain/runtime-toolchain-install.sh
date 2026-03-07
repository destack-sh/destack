#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./scripts/toolchain/versions.sh
source "${script_directory}/versions.sh"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
host_kernel="$(runtime_host_kernel)"

runtime_require_command rustup "missing required command: rustup" >&2 || exit 1
runtime_require_command cargo "missing required command: cargo" >&2 || exit 1

# install and activate the pinned rust toolchain
if ! rustup toolchain list | grep -E "^${DESTACK_RUST_TOOLCHAIN}( |$)" >/dev/null; then
	echo "installing rust toolchain: ${DESTACK_RUST_TOOLCHAIN}"
	rustup toolchain install "${DESTACK_RUST_TOOLCHAIN}" --profile minimal
fi
rustup default "${DESTACK_RUST_TOOLCHAIN}"

# host sdk tools
if [ "${host_kernel}" = "Darwin" ]; then
	if [ -z "$(runtime_command_path xcrun)" ]; then
		echo "missing xcrun: install xcode command line tools" >&2
		echo "run: xcode-select --install" >&2
		exit 1
	fi
fi

# clippy is required by runtime check lanes
if ! rustup component list --installed | grep -E '^clippy(-|$)' >/dev/null 2>&1; then
	echo "installing rust component: clippy"
	rustup component add clippy
fi

# runtime target matrix
runtime_ensure_rust_target aarch64-linux-android

if [ "${host_kernel}" = "Darwin" ]; then
	runtime_ensure_rust_target aarch64-apple-ios
fi

# android sdk and ndk
if ndk_root="$("${script_directory}"/resolve-android-ndk-root.sh 2>/dev/null)"; then
	echo "android ndk already available: ${ndk_root}"
else
	if [ "${host_kernel}" = "Darwin" ]; then
		default_android_sdk_root="${HOME}/Library/Android/sdk"
	else
		default_android_sdk_root="${HOME}/Android/Sdk"
	fi

	# install linux host dependencies for android sdk setup
	if [ "${host_kernel}" = "Linux" ] && ! command -v curl >/dev/null 2>&1; then
		echo "installing curl for android sdk setup"
		runtime_linux_install_package curl
	fi
	if [ "${host_kernel}" = "Linux" ] && ! command -v unzip >/dev/null 2>&1; then
		echo "installing unzip for android sdk setup"
		runtime_linux_install_package unzip
	fi
	if [ "${host_kernel}" = "Linux" ] && ! command -v java >/dev/null 2>&1; then
		echo "installing openjdk for android sdk setup"
		runtime_linux_install_package openjdk-17-jre-headless
	fi

	if ! command -v curl >/dev/null 2>&1 || ! command -v unzip >/dev/null 2>&1; then
		echo "android ndk is missing and curl/unzip are required to install it" >&2
		echo "install curl and unzip, then re-run: just language/install-toolchain" >&2
		exit 1
	fi
	if ! command -v java >/dev/null 2>&1; then
		echo "android ndk is missing and java is required to run sdkmanager" >&2
		echo "install one jre or jdk and re-run: just language/install-toolchain" >&2
		exit 1
	fi

	echo "installing android sdk cmdline tools and ndk"
	ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-${default_android_sdk_root}}" \
		"${repository_root}/.github/scripts/install-runtime-android-ndk.sh"
fi

# weston is required for the linux wayland display lane
if [ "${host_kernel}" = "Linux" ] && [ -z "$(runtime_command_path weston)" ]; then
	echo "installing weston for the linux wayland display lane"
	if ! runtime_linux_install_package weston; then
		echo "failed to install weston automatically"
		echo "install weston manually, then re-run: just language/install-toolchain"
		exit 1
	fi
fi

# install shellcheck for runtime toolchain lint lane
if [ "${host_kernel}" = "Linux" ] && [ -z "$(runtime_command_path shellcheck)" ]; then
	echo "installing shellcheck for runtime toolchain lint lane"
	runtime_linux_install_package shellcheck
fi

"${script_directory}/runtime-toolchain-doctor.sh"
