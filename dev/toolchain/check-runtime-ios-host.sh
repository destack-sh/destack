#!/usr/bin/env bash
set -euo pipefail

script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "${script_directory}/../.." && pwd)

if ! command -v xcodebuild >/dev/null 2>&1; then
	echo "missing xcodebuild: install xcode command line tools"
	exit 1
fi

destinations=$(
	cd "${repo_root}/language/runtime/apple"
	xcodebuild -scheme RuntimeHostIOS -showdestinations 2>/dev/null || true
)

if printf '%s\n' "${destinations}" | grep -Eq 'error:iOS .* is not installed'; then
	echo "missing iOS platform bundle: install the iPhoneOS platform components in Xcode"
	exit 1
fi

(
	cd "${repo_root}/language/runtime/apple"
	xcodebuild -scheme RuntimeHostIOS -destination "generic/platform=iOS" build
)
