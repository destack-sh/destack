#!/usr/bin/env bash
set -euo pipefail

version="$(cat VERSION.txt)"

if ! git rev-parse --verify "v${version}" >/dev/null 2>&1; then
	echo "error: missing local release tag v${version}" >&2
	echo "run: just release <major|minor|patch>" >&2
	exit 1
fi

git push origin main
git push origin "v${version}"
