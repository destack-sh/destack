#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./dev/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

find_android_gradle_command() {
	if [ -x "${repo_root}/language/runtime/android/gradlew" ]; then
		printf '%s\n' "./gradlew"
		return 0
	fi

	if command -v gradle >/dev/null 2>&1; then
		printf '%s\n' "gradle"
		return 0
	fi

	return 1
}

java_command="$(runtime_java_path || true)"
if [ -z "${java_command}" ]; then
	echo "missing java: install one jdk or jre to run android host checks"
	exit 1
fi

java_bin_directory="$(cd "$(dirname "${java_command}")" && pwd)"
java_home_directory="$(cd "${java_bin_directory}/.." && pwd)"
export JAVA_HOME="${JAVA_HOME:-${java_home_directory}}"
export PATH="${java_bin_directory}:${PATH}"

if sdk_root="$("${script_directory}/resolve-android-sdk-root.sh" 2>/dev/null)"; then
	export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-${sdk_root}}"
	export ANDROID_HOME="${ANDROID_HOME:-${sdk_root}}"
else
	echo "missing android sdk root"
	echo "run: just language/install-runtime-android-ndk"
	echo "or set ANDROID_SDK_ROOT"
	exit 1
fi

gradle_command="$(find_android_gradle_command || true)"
if [ -z "${gradle_command}" ]; then
	echo "missing gradle: install gradle or add language/runtime/android/gradlew"
	exit 1
fi

if [ "${gradle_command}" = "./gradlew" ]; then
	(
		cd "${repo_root}/language/runtime/android"
		./gradlew :kotlin:assembleDebug :kotlin:testDebugUnitTest
	)
else
	"${gradle_command}" \
		-p "${repo_root}/language/runtime/android" \
		:kotlin:assembleDebug \
		:kotlin:testDebugUnitTest
fi
