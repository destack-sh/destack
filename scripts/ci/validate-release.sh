#!/usr/bin/env bash
set -euo pipefail

version="$(cat VERSION.txt)"
release_tag="${1:-}"

if [ -z "${release_tag}" ]; then
	release_tag="v${version}"
fi

bash scripts/ci/validate-release-tag-version.sh "${release_tag}"
cargo run --release -p destack_cli -- dev version check
bash scripts/ci/validate-release-changelog.sh "${version}"
