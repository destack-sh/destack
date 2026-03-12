#!/usr/bin/env bash
set -euo pipefail

cli_artifacts_directory="${DESTACK_CLI_ARTIFACTS:-release-cli-assets}"

just build

if [ -d "${cli_artifacts_directory}" ]; then
	just app/stage-cli-binaries-from-artifacts "$(cat VERSION.txt)" "${cli_artifacts_directory}"
else
	if [ -z "${DESTACK_RELEASE_TARGETS:-}" ]; then
		host_target="$(rustc -vV | awk '/^host: / { print $2 }')"
		case "${host_target}" in
		aarch64-apple-darwin | x86_64-apple-darwin | aarch64-unknown-linux-gnu | x86_64-unknown-linux-gnu | x86_64-pc-windows-msvc)
			export DESTACK_RELEASE_TARGETS="${host_target}"
			;;
		*)
			echo "error: unsupported host target for default local publish: ${host_target}" >&2
			echo "set DESTACK_RELEASE_TARGETS explicitly to one or more supported targets" >&2
			exit 1
			;;
		esac
	fi

	just app/build-cli-binaries
	(cd app/cli && npm run stage:binaries)
fi

just app/validate-cli-publish
just library/publish ""
just app/publish ""
just bridge/publish ""
just template/publish-create-destack-live
