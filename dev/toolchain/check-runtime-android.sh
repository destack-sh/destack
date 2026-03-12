#!/usr/bin/env bash
set -euo pipefail

api_level="${ANDROID_API_LEVEL:-24}"
script_directory="$(cd "$(dirname "$0")" && pwd)"
ndk_root=""

# resolve ndk root through explicit env or host sdk defaults
if ndk_root="$("${script_directory}/resolve-android-ndk-root.sh" 2>/dev/null)"; then
	:
else
	echo "missing android ndk root"
	echo "run: just language/install-toolchain"
	echo "or set ANDROID_NDK_ROOT"
	exit 1
fi
echo "android ndk root: ${ndk_root}"

toolchain_bin="$("${script_directory}/android-ndk-toolchain-bin.sh" "${ndk_root}")"

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

# ensure target std is available before cargo check
if command -v rustup >/dev/null 2>&1; then
	rustup target add aarch64-linux-android >/dev/null
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
run_android_command cargo test -p destack_runtime --target aarch64-linux-android --no-run
