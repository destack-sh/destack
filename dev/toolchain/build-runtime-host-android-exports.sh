#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./dev/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

api_level="${ANDROID_API_LEVEL:-24}"

if ndk_root="$("${script_directory}/resolve-android-ndk-root.sh" 2>/dev/null)"; then
	:
else
	echo "missing android ndk root"
	echo "run: just language/install-runtime-android-ndk"
	echo "or set ANDROID_NDK_ROOT"
	exit 1
fi

toolchain_bin="$("${script_directory}/android-ndk-toolchain-bin.sh" "${ndk_root}")"
target_triple="aarch64-linux-android"
android_abi="arm64-v8a"
linker="${toolchain_bin}/aarch64-linux-android${api_level}-clang"
archiver="${toolchain_bin}/llvm-ar"
ranlib="${toolchain_bin}/llvm-ranlib"
target_directory="${repo_root}/language/runtime/android/kotlin/.build/rust-target"
output_directory="${repo_root}/language/runtime/android/kotlin/.build/rust/${android_abi}"
output_library="${output_directory}/libdestack_runtime_abi.a"
built_library="${target_directory}/${target_triple}/debug/libdestack_runtime_abi.a"

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

runtime_ensure_rust_target "${target_triple}"
mkdir -p "${output_directory}"

LC_ALL=C \
	LANG=C \
	LC_CTYPE=C \
	RUSTFLAGS="-C relocation-model=pic" \
	CARGO_INCREMENTAL=0 \
	CFLAGS_aarch64_linux_android="-fPIC" \
	CXXFLAGS_aarch64_linux_android="-fPIC" \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${linker}" \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="${archiver}" \
	TARGET_RANLIB="${ranlib}" \
	CC_aarch64_linux_android="${linker}" \
	AR_aarch64_linux_android="${archiver}" \
	RANLIB_aarch64_linux_android="${ranlib}" \
	CXX_aarch64_linux_android="${linker}" \
	RANLIB="${ranlib}" \
	CARGO_TARGET_DIR="${target_directory}" \
	cargo build \
		--manifest-path "${repo_root}/language/runtime/abi/Cargo.toml" \
		--target "${target_triple}"

cp "${built_library}" "${output_library}"
