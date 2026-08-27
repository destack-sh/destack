#!/usr/bin/env bash
set -euo pipefail

version="$(just version)"
stability="$(just stability)"
release_tag="${1:-}"

case "${stability}" in
	experimental | alpha | beta | stable) ;;
	*)
		echo "invalid release stability '${stability}'" >&2
		exit 1
		;;
esac

if [ -z "${release_tag}" ]; then
	release_tag="v${version}"
fi

bash dev/ci/validate-release-tag-version.sh "${release_tag}"
cargo run --release -p destack_cli -- dev version check
