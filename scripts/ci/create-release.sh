#!/usr/bin/env bash
set -euo pipefail

kind="${1:?missing version bump kind}"

just bump "${kind}"
just generate-release-changelog

version="$(cat VERSION.txt)"
just validate-release "v${version}"

release_commit_message="chore(all): bump version to ${version}"
git add -A
git commit -m "${release_commit_message}"
git tag -a "v${version}" -m "Release v${version}"

echo ""
echo "Release v${version} created locally."
echo "To publish:"
echo "  just push-release"
echo "  just publish-release"
echo "  just publish-release-local   # uses release-cli-assets when present, else host target"
