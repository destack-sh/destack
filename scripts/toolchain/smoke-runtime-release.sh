#!/usr/bin/env bash
set -euo pipefail

target="${1:-}"

if [ -z "${target}" ]; then
	target="$(rustc -vV | awk '/^host: / { print $2 }')"
fi

if [ -z "${target}" ]; then
	echo "missing target and failed to resolve rust host target" >&2
	exit 1
fi

if command -v rustup >/dev/null 2>&1; then
	rustup target add "${target}" >/dev/null
fi

executable_suffix=""
case "${target}" in
	*-windows-*) executable_suffix=".exe" ;;
esac

release_directory="target/${target}/release"
release_executable="${release_directory}/runtime-smoke${executable_suffix}"

cargo build --release -p destack_runtime --bin runtime-smoke --target "${target}"
"./${release_executable}"
