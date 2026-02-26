#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
host_kernel="$(uname -s)"

ensure_command() {
    command_name="$1"
    if ! command -v "${command_name}" >/dev/null 2>&1; then
        echo "missing required command: ${command_name}" >&2
        exit 1
    fi
}

ensure_rust_target() {
    target="$1"
    if rustup target list --installed | grep -Fx "${target}" >/dev/null 2>&1; then
        echo "rust target already installed: ${target}"
        return 0
    fi

    echo "installing rust target: ${target}"
    rustup target add "${target}"
}

ensure_command rustup
ensure_command cargo

# host sdk tools
if [ "${host_kernel}" = "Darwin" ]; then
    if ! command -v xcrun >/dev/null 2>&1; then
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
ensure_rust_target wasm32-wasip1
ensure_rust_target x86_64-pc-windows-gnu
ensure_rust_target aarch64-linux-android

if [ "${host_kernel}" = "Darwin" ]; then
    ensure_rust_target aarch64-apple-ios
fi

if [ "${host_kernel}" = "Linux" ]; then
    ensure_rust_target x86_64-unknown-linux-gnu
    ensure_rust_target aarch64-unknown-linux-gnu
fi

# android sdk and ndk
if ndk_root="$(${script_directory}/resolve-android-ndk-root.sh 2>/dev/null)"; then
    echo "android ndk already available: ${ndk_root}"
else
    if [ "${host_kernel}" = "Darwin" ]; then
        default_android_sdk_root="${HOME}/Library/Android/sdk"
    else
        default_android_sdk_root="${HOME}/Android/Sdk"
    fi

    if ! command -v curl >/dev/null 2>&1 || ! command -v unzip >/dev/null 2>&1; then
        echo "android ndk is missing and curl/unzip are required to install it" >&2
        echo "install curl and unzip, then re-run: just runtime-toolchain-bootstrap" >&2
        exit 1
    fi
    if ! command -v java >/dev/null 2>&1; then
        echo "android ndk is missing and java is required to run sdkmanager" >&2
        echo "install one jre or jdk and re-run: just runtime-toolchain-bootstrap" >&2
        exit 1
    fi

    echo "installing android sdk cmdline tools and ndk"
    ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-${default_android_sdk_root}}" \
        "${repository_root}/.github/scripts/install-android-ndk.sh"
fi

# host package manager guidance for optional system tools
if ! command -v zig >/dev/null 2>&1; then
    echo "zig is missing: install zig to run windows gnu and cross runtime lanes"
fi
if [ "${host_kernel}" = "Linux" ] && ! command -v wine >/dev/null 2>&1; then
    echo "wine is missing: install wine to run windows gnu executable runtime tests"
fi
if [ "${host_kernel}" = "Linux" ] \
    && command -v pkg-config >/dev/null 2>&1 \
    && ! pkg-config --exists dbus-1 >/dev/null 2>&1; then
    echo "dbus pkg-config is missing: install libdbus-1-dev and pkg-config for linux gnu cross lane"
fi

"${script_directory}/runtime-toolchain-doctor.sh"
