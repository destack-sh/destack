#!/usr/bin/env bash
set -euo pipefail

just next-version

version="$(just version)"
stability="$(just stability)"
just validate-release "v${version}"

release_commit_message="chore(all): release ${version} ${stability}"
git add -A
git commit -m "${release_commit_message}"
git tag -a "v${version}" -m "Release v${version} (${stability})"

echo ""
echo "Release v${version} created locally."
echo "To publish:"
echo "  just release-push"
echo "  just publish-release"
echo "  just publish-release-local   # uses release-cli-assets when present, else host target"
