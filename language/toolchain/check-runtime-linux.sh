#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

if [ "$(runtime_host_kernel)" = "Linux" ]; then
	cd "${repo_root}/language"
	runtime_run_full_runtime_crate_lane cargo
	exit 0
fi

container_engine="$(runtime_host_container_engine)"
if [ -z "${container_engine}" ]; then
	echo "missing container runtime: install docker or podman to run the local linux runtime lane"
	exit 1
fi

if ! runtime_host_container_ready "${container_engine}"; then
	echo "linux runtime lane requires one running ${container_engine} daemon or machine"
	exit 1
fi

toolchain_channel="$(runtime_rust_toolchain_channel "${repo_root}")"
if [ -z "${toolchain_channel}" ]; then
	echo "failed to determine rust toolchain channel from rust-toolchain.toml"
	exit 1
fi

runtime_run_linux_container_runtime_lane "${container_engine}" "${repo_root}" "${toolchain_channel}"
