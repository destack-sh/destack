#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
cd "${repository_root}"

tag="${1:-${GITHUB_REF_NAME:-}}"

# require a tag argument or a github tag ref name
if [ -z "${tag}" ]; then
	echo "missing release tag: pass vX.Y.Z or set GITHUB_REF_NAME" >&2
	exit 1
fi

# require the standard release tag prefix
if [[ "${tag}" != v* ]]; then
	echo "invalid release tag '${tag}': expected vX.Y.Z" >&2
	exit 1
fi

version="$(tr -d '[:space:]' <VERSION.txt)"
tag_version="${tag#v}"

# require strict semver style for root version
if ! [[ "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "invalid VERSION.txt '${version}': expected X.Y.Z" >&2
	exit 1
fi

# require tag and version file to match exactly
if [ "${tag_version}" != "${version}" ]; then
	echo "release tag/version mismatch: tag=${tag}, VERSION.txt=${version}" >&2
	exit 1
fi

echo "validated release tag ${tag} matches VERSION.txt (${version})"
