#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

windows_target="$(runtime_windows_gnu_target)"

run_cross_command() {
	runtime_set_standard_environment
	"$@"
}

# ensure target std is available before cargo check
runtime_ensure_rust_target "${windows_target}"
runtime_ensure_rust_target x86_64-unknown-linux-gnu
runtime_ensure_rust_target aarch64-unknown-linux-gnu

# require host linker toolchain for windows gnu compilation
runtime_windows_gnu_require_build_toolchain
runtime_require_command zig "missing zig: install zig to build linux gnu cross targets" || exit 1

# windows gnu cross check is always part of this lane
runtime_windows_gnu_run cargo check -p destack_runtime --target "${windows_target}"

# linux gnu cross checks are now host-agnostic:
# keyring dbus is built vendored, so no host dbus pkg-config metadata is required
run_cross_command cargo check -p destack_runtime --target x86_64-unknown-linux-gnu
run_cross_command cargo check -p destack_runtime --target aarch64-unknown-linux-gnu
