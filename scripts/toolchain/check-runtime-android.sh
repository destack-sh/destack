#!/usr/bin/env bash
set -euo pipefail

api_level="${ANDROID_API_LEVEL:-24}"
ndk_root="${ANDROID_NDK_ROOT:-${ANDROID_NDK_HOME:-}}"

if [ -z "${ndk_root}" ]; then
    echo "missing ANDROID_NDK_ROOT (or ANDROID_NDK_HOME)"
    exit 1
fi

toolchain_bin="$("$(dirname "$0")/android-ndk-toolchain-bin.sh" "${ndk_root}")"

linker="${toolchain_bin}/aarch64-linux-android${api_level}-clang"
archiver="${toolchain_bin}/llvm-ar"
ranlib="${toolchain_bin}/llvm-ranlib"

if [ ! -x "${linker}" ]; then
    echo "missing android linker: ${linker}"
    exit 1
fi

if [ ! -x "${archiver}" ]; then
    echo "missing android archiver: ${archiver}"
    exit 1
fi

if [ ! -x "${ranlib}" ]; then
    echo "missing android ranlib: ${ranlib}"
    exit 1
fi

run_android_command() {
    LC_ALL=C \
    LANG=C \
    LC_CTYPE=C \
    CARGO_INCREMENTAL=0 \
    CRATE_CC_NO_DEFAULTS=1 \
    CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${linker}" \
    CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="${archiver}" \
    TARGET_RANLIB="${ranlib}" \
    CC_aarch64_linux_android="${linker}" \
    AR_aarch64_linux_android="${archiver}" \
    RANLIB_aarch64_linux_android="${ranlib}" \
    CXX_aarch64_linux_android="${linker}" \
    RANLIB="${ranlib}" \
    "$@"
}

run_android_command cargo check -p destack_runtime --target aarch64-linux-android
run_android_command cargo clippy -p destack_runtime --target aarch64-linux-android --no-deps -- -D warnings
run_android_command cargo test -p destack_runtime --target aarch64-linux-android --no-run runtime::host::android::
