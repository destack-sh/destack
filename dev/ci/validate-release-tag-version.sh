#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
cd "${repository_root}"

tag="${1:-${GITHUB_REF_NAME:-}}"

# require a tag argument or a github tag ref name
if [ -z "${tag}" ]; then
	echo "missing release tag: pass vYEAR.MONTH.MICRO or set GITHUB_REF_NAME" >&2
	exit 1
fi

# require the canonical calendar version tag
if ! [[ "${tag}" =~ ^v[0-9]{4}\.([1-9]|1[0-2])\.[0-9]+$ ]]; then
	echo "invalid release tag '${tag}': expected vYEAR.MONTH.MICRO" >&2
	exit 1
fi

version="$(node -p 'JSON.parse(require("node:fs").readFileSync("destack.json", "utf8")).version')"
tag_version="${tag#v}"

# require the canonical calendar version
if ! [[ "${version}" =~ ^[0-9]{4}\.([1-9]|1[0-2])\.[0-9]+$ ]]; then
	echo "invalid destack.json version '${version}': expected YEAR.MONTH.MICRO" >&2
	exit 1
fi

# require tag and version file to match exactly
if [ "${tag_version}" != "${version}" ]; then
	echo "release tag/version mismatch: tag=${tag}, destack.json=${version}" >&2
	exit 1
fi

echo "validated release tag ${tag} matches destack.json (${version})"
