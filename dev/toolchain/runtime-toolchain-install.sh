#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./dev/toolchain/versions.sh
source "${script_directory}/versions.sh"

# shellcheck source=./dev/toolchain/lib/runtime-common.sh
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
runtime_ensure_rust_target aarch64-unknown-linux-gnu
runtime_ensure_rust_target x86_64-pc-windows-gnu

if [ -z "$(runtime_command_path zig)" ]; then
	if [ "${host_kernel}" = "Linux" ]; then
		echo "installing zig for runtime cross target lanes"
		if ! runtime_linux_install_package zig; then
			echo "failed to install zig automatically" >&2
			echo "install zig manually, then re-run: just language/install-toolchain" >&2
			exit 1
		fi
	else
		echo "missing zig: install zig to cross check the runtime linux aarch64 and windows gnu lanes" >&2
		echo "install zig, then re-run: just language/install-toolchain" >&2
		exit 1
	fi
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
