#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment
runtime_ensure_rust_target aarch64-unknown-linux-gnu

zig_path="$(runtime_command_path zig)"
if [ -z "${zig_path}" ]; then
	echo "missing zig: install zig to cross check the runtime linux aarch64 lane"
	exit 1
fi

linker_path="${script_directory}/cc-aarch64-linux-gnu"

run_linux_aarch64_command() {
	env \
		CC_aarch64_unknown_linux_gnu="${linker_path}" \
		CXX_aarch64_unknown_linux_gnu="${linker_path}" \
		"CC_aarch64-unknown-linux-gnu=${linker_path}" \
		"CXX_aarch64-unknown-linux-gnu=${linker_path}" \
		CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${linker_path}" \
		"$@"
}

run_linux_aarch64_command cargo check -p tspp_runtime --target aarch64-unknown-linux-gnu
run_linux_aarch64_command cargo clippy -p tspp_runtime --target aarch64-unknown-linux-gnu --no-deps -- -D warnings
run_linux_aarch64_command cargo test -p tspp_runtime --target aarch64-unknown-linux-gnu --no-run
