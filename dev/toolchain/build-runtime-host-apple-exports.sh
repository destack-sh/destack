#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./dev/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

architecture="$(uname -m)"

if [ "${architecture}" = "arm64" ]; then
	target_triple="aarch64-apple-darwin"
elif [ "${architecture}" = "x86_64" ]; then
	target_triple="x86_64-apple-darwin"
else
	echo "unsupported apple host architecture: ${architecture}"
	exit 1
fi

runtime_ensure_rust_target "${target_triple}"

target_directory="${repo_root}/language/runtime/apple/.build/rust-target"
output_directory="${repo_root}/language/runtime/apple/.build/rust"
output_library="${output_directory}/libdestack_runtime_abi.a"
built_library="${target_directory}/${target_triple}/debug/libdestack_runtime_abi.a"

mkdir -p "${output_directory}"

LC_ALL=C \
	LANG=C \
	LC_CTYPE=C \
	MACOSX_DEPLOYMENT_TARGET=13.0 \
	RUSTFLAGS="-C link-arg=-mmacosx-version-min=13.0" \
	CARGO_INCREMENTAL=0 \
	CARGO_TARGET_DIR="${target_directory}" \
	cargo build \
		--manifest-path "${repo_root}/language/runtime/abi/Cargo.toml" \
		--target "${target_triple}"

cp "${built_library}" "${output_library}"
