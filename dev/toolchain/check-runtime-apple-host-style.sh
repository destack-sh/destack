#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./dev/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

if [ "$(runtime_host_kernel)" != "Darwin" ]; then
	echo "skipping apple host style checks: darwin host required"
	exit 0
fi

swift format lint --strict --parallel --recursive \
	"${repo_root}/language/runtime/apple/Package.swift" \
	"${repo_root}/language/runtime/apple/core/Sources/RuntimeHostAppleCore" \
	"${repo_root}/language/runtime/apple/core/Tests/RuntimeHostAppleCoreTests" \
	"${repo_root}/language/runtime/apple/ios/Sources/RuntimeHostIOS" \
	"${repo_root}/language/runtime/apple/ios/Tests/RuntimeHostIOSTests" \
	"${repo_root}/language/runtime/apple/macos/Sources/RuntimeHostMacOS" \
	"${repo_root}/language/runtime/apple/macos/Tests/RuntimeHostMacOSTests"
