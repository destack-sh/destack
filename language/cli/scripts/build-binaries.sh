#!/usr/bin/env bash
set -euo pipefail

cargo_bin="cargo"
rustup_bin="rustup"

if [ "${OS:-}" = "Windows_NT" ] && command -v cargo.exe >/dev/null 2>&1; then
	cargo_bin="cargo.exe"
fi

if [ "${OS:-}" = "Windows_NT" ] && command -v rustup.exe >/dev/null 2>&1; then
	rustup_bin="rustup.exe"
fi

targets_input="${DESTACK_RELEASE_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc}"
selected_targets=()

for token in ${targets_input}; do
	case "${token}" in
	darwin-arm64 | aarch64-apple-darwin) selected_targets+=("aarch64-apple-darwin") ;;
	darwin-x64 | x86_64-apple-darwin) selected_targets+=("x86_64-apple-darwin") ;;
	linux-arm64-gnu | aarch64-unknown-linux-gnu) selected_targets+=("aarch64-unknown-linux-gnu") ;;
	linux-x64-gnu | x86_64-unknown-linux-gnu) selected_targets+=("x86_64-unknown-linux-gnu") ;;
	win32-x64-msvc | x86_64-pc-windows-msvc) selected_targets+=("x86_64-pc-windows-msvc") ;;
	*)
		echo "error: unsupported DESTACK_RELEASE_TARGETS entry: ${token}" >&2
		exit 1
		;;
	esac
done

built_targets=""
for target_triple in "${selected_targets[@]}"; do
	if [[ " ${built_targets} " == *" ${target_triple} "* ]]; then
		continue
	fi

	built_targets="${built_targets} ${target_triple}"
	"${rustup_bin}" target add "${target_triple}"
	"${cargo_bin}" build --release -p tspp_cli --target "${target_triple}"
done
