#!/usr/bin/env bash
set -euo pipefail

if ! command -v xcrun >/dev/null 2>&1; then
	echo "missing xcrun: install xcode command line tools"
	exit 1
fi

sdk_path="$(xcrun --sdk iphoneos --show-sdk-path)"
platform_path="$(xcrun --sdk iphoneos --show-sdk-platform-path)"
deployment_target="${IOS_DEPLOYMENT_TARGET:-13.0}"
clang_path="$(xcrun --sdk iphoneos --find clang)"
archiver_path="$(xcrun --sdk iphoneos --find ar)"
ranlib_path="$(xcrun --sdk iphoneos --find ranlib)"
cross_top="${platform_path}/Developer"
cross_sdk="$(basename "${sdk_path}")"

if [ -z "${sdk_path}" ] || [ ! -d "${sdk_path}" ]; then
	echo "missing iphoneos sdk path from xcrun"
	exit 1
fi

if [ -z "${platform_path}" ] || [ ! -d "${platform_path}" ]; then
	echo "missing iphoneos platform path from xcrun"
	exit 1
fi

# ensure target std is available before cargo check
if command -v rustup >/dev/null 2>&1; then
	rustup target add aarch64-apple-ios >/dev/null
fi

run_ios_command() {
	local sdk_flags

	sdk_flags="-isysroot ${sdk_path} -miphoneos-version-min=${deployment_target}"

	env \
		LC_ALL=C \
		LANG=C \
		LC_CTYPE=C \
		CARGO_INCREMENTAL=0 \
		CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="${clang_path}" \
		CC_aarch64_apple_ios="${clang_path}" \
		CXX_aarch64_apple_ios="${clang_path}" \
		AR_aarch64_apple_ios="${archiver_path}" \
		RANLIB_aarch64_apple_ios="${ranlib_path}" \
		"CC_aarch64-apple-ios=${clang_path}" \
		"CXX_aarch64-apple-ios=${clang_path}" \
		"AR_aarch64-apple-ios=${archiver_path}" \
		"RANLIB_aarch64-apple-ios=${ranlib_path}" \
		CROSS_TOP="${cross_top}" \
		CROSS_SDK="${cross_sdk}" \
		CFLAGS="${sdk_flags}" \
		CXXFLAGS="${sdk_flags}" \
		CFLAGS_aarch64_apple_ios="${sdk_flags}" \
		CXXFLAGS_aarch64_apple_ios="${sdk_flags}" \
		"CFLAGS_aarch64-apple-ios=${sdk_flags}" \
		"CXXFLAGS_aarch64-apple-ios=${sdk_flags}" \
		SDKROOT="${sdk_path}" \
		IPHONEOS_DEPLOYMENT_TARGET="${deployment_target}" \
		"$@"
}

run_ios_command cargo check -p destack_runtime --target aarch64-apple-ios
run_ios_command cargo clippy -p destack_runtime --target aarch64-apple-ios --no-deps -- -D warnings
run_ios_command cargo test -p destack_runtime --target aarch64-apple-ios --no-run
