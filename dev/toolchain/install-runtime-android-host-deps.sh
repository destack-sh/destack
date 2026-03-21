#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./dev/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

ensure_android_host_dependency() {
	command_name="$1"
	package_name="$2"
	missing_message="$3"

	runtime_require_or_auto_install_command \
		"${command_name}" \
		"${package_name}" \
		"${missing_message}" \
		"just language/install-runtime-android-host-deps"
}

# ensure host tools needed by the android sdk installer script
ensure_android_host_dependency \
	curl \
	curl \
	"missing curl: install curl to fetch android commandline tools" || exit 1
ensure_android_host_dependency \
	unzip \
	unzip \
	"missing unzip: install unzip to extract android commandline tools" || exit 1
ensure_android_host_dependency \
	java \
	openjdk@17 \
	"missing java: install one jre or jdk to run sdkmanager" || exit 1
ensure_android_host_dependency \
	gradle \
	gradle \
	"missing gradle: install gradle or add language/runtime/android/gradlew" || exit 1
