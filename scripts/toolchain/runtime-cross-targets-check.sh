#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

host_kernel="$(runtime_host_kernel)"
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

# windows gnu cross check is always part of this lane
runtime_windows_gnu_run cargo check -p destack_runtime --target "${windows_target}"

# linux gnu cross checks depend on dbus pkg-config for keyring secret service support
if [ "${host_kernel}" = "Linux" ]; then
    runtime_require_command pkg-config "missing pkg-config: install pkg-config and libdbus-1-dev"

    if ! pkg-config --exists dbus-1 >/dev/null 2>&1; then
        echo "missing dbus pkg-config metadata: install libdbus-1-dev"
        exit 1
    fi

    run_cross_command cargo check -p destack_runtime --target x86_64-unknown-linux-gnu
    run_cross_command cargo check -p destack_runtime --target aarch64-unknown-linux-gnu
    exit 0
fi

# non-linux hosts cannot reliably resolve linux dbus sysroots for keyring
if [ "${DESTACK_REQUIRE_LINUX_GNU_CROSS:-0}" = "1" ]; then
    echo "linux gnu cross checks require one linux host with dbus pkg-config support"
    echo "run this lane on linux or unset DESTACK_REQUIRE_LINUX_GNU_CROSS"
    exit 1
fi

echo "skipping linux gnu cross checks on ${host_kernel}: run on linux for full validation"
