#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
cd "${repository_root}"

version="${1:-$(tr -d '[:space:]' <VERSION.txt)}"
changelog_path="${repository_root}/CHANGELOG.md"
dart_package_directory="${repository_root}/bridge/dart"
dart_changelog_path="${dart_package_directory}/CHANGELOG.md"
root_license_path="${repository_root}/LICENSE.txt"
dart_license_path="${dart_package_directory}/LICENSE"

# require strict semver format
if ! [[ "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "invalid version '${version}': expected X.Y.Z" >&2
	exit 1
fi

# require changelog file presence
if [ ! -f "${changelog_path}" ]; then
	echo "missing CHANGELOG.md" >&2
	exit 1
fi

# require a section heading for the release version
if ! rg -n "^## \\[${version}\\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" "${changelog_path}" >/dev/null; then
	echo "missing changelog entry for ${version} in CHANGELOG.md" >&2
	exit 1
fi

# require dart package changelog to include the release version
if [ -d "${dart_package_directory}" ]; then
	if [ ! -f "${dart_changelog_path}" ]; then
		echo "missing bridge/dart/CHANGELOG.md" >&2
		exit 1
	fi

	if ! rg -n "^## ${version} - [0-9]{4}-[0-9]{2}-[0-9]{2}$" "${dart_changelog_path}" >/dev/null; then
		echo "missing changelog entry for ${version} in bridge/dart/CHANGELOG.md" >&2
		exit 1
	fi

	if [ -f "${root_license_path}" ]; then
		if [ ! -f "${dart_license_path}" ]; then
			echo "missing bridge/dart/LICENSE" >&2
			exit 1
		fi

		if ! cmp -s "${root_license_path}" "${dart_license_path}"; then
			echo "bridge/dart/LICENSE is out of sync with LICENSE.txt" >&2
			exit 1
		fi
	fi
fi

echo "validated changelog entry for ${version}"
