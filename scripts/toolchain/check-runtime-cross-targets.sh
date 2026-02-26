#!/usr/bin/env bash
set -euo pipefail

host_kernel="$(uname -s)"

run_cross_command() {
    LC_ALL=C LANG=C CARGO_INCREMENTAL=0 "$@"
}

# ensure target std is available before cargo check
if command -v rustup >/dev/null 2>&1; then
    rustup target add x86_64-pc-windows-gnu >/dev/null
    rustup target add x86_64-unknown-linux-gnu >/dev/null
    rustup target add aarch64-unknown-linux-gnu >/dev/null
fi

# zig is required by the cross linker wrappers in .cargo/config.toml
if ! command -v zig >/dev/null 2>&1; then
    echo "missing zig: install zig to run runtime cross-target checks"
    exit 1
fi

# windows gnu cross check is always part of this lane
run_cross_command cargo check -p destack_runtime --target x86_64-pc-windows-gnu

# linux gnu cross checks depend on dbus pkg-config for keyring secret service support
if [ "${host_kernel}" = "Linux" ]; then
    if ! command -v pkg-config >/dev/null 2>&1; then
        echo "missing pkg-config: install pkg-config and libdbus-1-dev"
        exit 1
    fi

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
