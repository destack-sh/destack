#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
cd "${repository_root}"

version="${1:-$(tr -d '[:space:]' <VERSION.txt)}"
changelog_path="${repository_root}/CHANGELOG.md"

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

echo "validated changelog entry for ${version}"
