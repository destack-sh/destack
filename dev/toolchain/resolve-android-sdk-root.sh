#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "$0")/../.." && pwd)"

resolve_existing_directory() {
	path="$1"
	if [ -n "${path}" ] && [ -d "${path}" ]; then
		printf '%s\n' "${path}"
		return 0
	fi

	return 1
}

resolve_sdk_directory() {
	sdk_directory="$1"

	if [ ! -d "${sdk_directory}" ]; then
		return 1
	fi

	if [ -d "${sdk_directory}/platforms" ] || [ -d "${sdk_directory}/cmdline-tools" ] || [ -d "${sdk_directory}/ndk" ]; then
		printf '%s\n' "${sdk_directory}"
		return 0
	fi

	return 1
}

resolve_from_local_properties() {
	local_properties_path="${repository_root}/language/runtime/android/local.properties"
	if [ ! -f "${local_properties_path}" ]; then
		return 1
	fi

	sdk_directory_line="$(sed -n 's/^sdk\.dir=//p' "${local_properties_path}" | tail -n1 || true)"
	if [ -z "${sdk_directory_line}" ]; then
		return 1
	fi

	sdk_directory="${sdk_directory_line//\\:/:}"
	sdk_directory="${sdk_directory//\\\\/\\}"

	resolve_sdk_directory "${sdk_directory}"
}

resolve_from_default_sdk_locations() {
	host_kernel="$(uname -s)"

	if [ "${host_kernel}" = "Darwin" ]; then
		resolve_sdk_directory "${HOME}/Library/Android/sdk"
		return "$?"
	fi

	if [ "${host_kernel}" = "Linux" ]; then
		if resolve_sdk_directory "${HOME}/Android/Sdk"; then
			return 0
		fi

		resolve_sdk_directory "${HOME}/.android-sdk"
		return "$?"
	fi

	if [ "${OS:-}" = "Windows_NT" ]; then
		resolve_sdk_directory "${LOCALAPPDATA:-}/Android/Sdk"
		return "$?"
	fi

	return 1
}

if resolved="$(resolve_existing_directory "${ANDROID_SDK_ROOT:-}")"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

if resolved="$(resolve_existing_directory "${ANDROID_HOME:-}")"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

if resolved="$(resolve_from_local_properties)"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

if resolved="$(resolve_from_default_sdk_locations)"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

echo "unable to resolve android sdk root" >&2
echo "set ANDROID_SDK_ROOT or install one android sdk under a standard host location" >&2
exit 1
