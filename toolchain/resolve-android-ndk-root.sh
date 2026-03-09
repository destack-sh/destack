#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "${script_directory}/.." && pwd)"

resolve_existing_directory() {
	path="$1"
	if [ -n "${path}" ] && [ -d "${path}" ]; then
		printf '%s\n' "${path}"
		return 0
	fi

	return 1
}

resolve_from_ndk_directory() {
	ndk_directory="$1"
	if [ ! -d "${ndk_directory}" ]; then
		return 1
	fi

	latest_version_directory="$(
		find "${ndk_directory}" -mindepth 1 -maxdepth 1 -type d -print 2>/dev/null | sort -V | tail -n1 || true
	)"
	if [ -n "${latest_version_directory}" ] && [ -d "${latest_version_directory}" ]; then
		printf '%s\n' "${latest_version_directory}"
		return 0
	fi

	return 1
}

resolve_from_sdk_directory() {
	sdk_directory="$1"
	if [ ! -d "${sdk_directory}" ]; then
		return 1
	fi

	if resolved="$(resolve_from_ndk_directory "${sdk_directory}/ndk")"; then
		printf '%s\n' "${resolved}"
		return 0
	fi

	if [ -d "${sdk_directory}/ndk-bundle" ]; then
		printf '%s\n' "${sdk_directory}/ndk-bundle"
		return 0
	fi

	return 1
}

resolve_from_local_properties() {
	local_properties_path="${repository_root}/local.properties"
	if [ ! -f "${local_properties_path}" ]; then
		return 1
	fi

	sdk_directory_line="$(sed -n 's/^sdk\.dir=//p' "${local_properties_path}" | tail -n1 || true)"
	if [ -z "${sdk_directory_line}" ]; then
		return 1
	fi

	sdk_directory="${sdk_directory_line//\\:/:}"
	sdk_directory="${sdk_directory//\\\\/\\}"

	resolve_from_sdk_directory "${sdk_directory}"
}

resolve_from_default_sdk_locations() {
	host_kernel="$(uname -s)"

	if [ "${host_kernel}" = "Darwin" ]; then
		if resolved="$(resolve_from_sdk_directory "${HOME}/Library/Android/sdk")"; then
			printf '%s\n' "${resolved}"
			return 0
		fi
		return 1
	fi

	if [ "${host_kernel}" = "Linux" ]; then
		if resolved="$(resolve_from_sdk_directory "${HOME}/Android/Sdk")"; then
			printf '%s\n' "${resolved}"
			return 0
		fi
		if resolved="$(resolve_from_sdk_directory "${HOME}/.android-sdk")"; then
			printf '%s\n' "${resolved}"
			return 0
		fi
		return 1
	fi

	if [ "${OS:-}" = "Windows_NT" ]; then
		if resolved="$(resolve_from_sdk_directory "${LOCALAPPDATA:-}/Android/Sdk")"; then
			printf '%s\n' "${resolved}"
			return 0
		fi
	fi

	return 1
}

# prefer explicit environment first
if resolved="$(resolve_existing_directory "${ANDROID_NDK_ROOT:-}")"; then
	printf '%s\n' "${resolved}"
	exit 0
fi
if resolved="$(resolve_existing_directory "${ANDROID_NDK_HOME:-}")"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

# then check repository local.properties sdk mapping
if resolved="$(resolve_from_local_properties)"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

# then check default host sdk locations
if resolved="$(resolve_from_default_sdk_locations)"; then
	printf '%s\n' "${resolved}"
	exit 0
fi

echo "unable to resolve android ndk root" >&2
echo "set ANDROID_NDK_ROOT or install one ndk under your android sdk" >&2
exit 1
