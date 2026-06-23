#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cli_directory="$(cd "${script_directory}/.." && pwd)"

if [ "$#" -gt 0 ]; then
	dry="$1"
else
	dry="--dry-run"
fi

resolve_package_directory() {
	local token="$1"
	case "${token}" in
	darwin-arm64 | aarch64-apple-darwin) printf '%s\n' "npm-darwin-arm64" ;;
	darwin-x64 | x86_64-apple-darwin) printf '%s\n' "npm-darwin-x64" ;;
	linux-arm64-gnu | aarch64-unknown-linux-gnu) printf '%s\n' "npm-linux-arm64-gnu" ;;
	linux-x64-gnu | x86_64-unknown-linux-gnu) printf '%s\n' "npm-linux-x64-gnu" ;;
	win32-x64-msvc | x86_64-pc-windows-msvc) printf '%s\n' "npm-win32-x64-msvc" ;;
	*)
		printf '%s\n' "error: unsupported DESTACK_RELEASE_TARGETS entry: ${token}" >&2
		exit 1
		;;
	esac
}

append_unique_package_directory() {
	local package_directory="$1"
	if [[ " ${selected_package_directories} " == *" ${package_directory} "* ]]; then
		return
	fi

	selected_package_directories="${selected_package_directories} ${package_directory}"
}

selected_package_directories=""
if [ -n "${DESTACK_RELEASE_TARGETS:-}" ]; then
	for target_token in ${DESTACK_RELEASE_TARGETS}; do
		package_directory="$(resolve_package_directory "${target_token}")"
		append_unique_package_directory "${package_directory}"
	done
fi

if [ -z "${selected_package_directories}" ]; then
	selected_package_directories="npm-darwin-arm64 npm-darwin-x64 npm-linux-arm64-gnu npm-linux-x64-gnu npm-win32-x64-msvc"
fi

for package_directory in ${selected_package_directories}; do
	(cd "${cli_directory}/${package_directory}" && npm publish "${dry}" --access public)
done
