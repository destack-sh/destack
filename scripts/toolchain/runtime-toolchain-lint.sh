#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_require_command shellcheck "missing shellcheck: install shellcheck to lint runtime toolchain scripts" >&2 || exit 1

runtime_toolchain_shell_scripts=(
    "${script_directory}"/*.sh
    "${script_directory}"/lib/*.sh
)

for script_path in "${runtime_toolchain_shell_scripts[@]}"; do
    bash -n "${script_path}"
done

cd "${repository_root}"
shellcheck -x "${runtime_toolchain_shell_scripts[@]}"
