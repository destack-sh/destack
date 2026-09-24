#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_require_or_auto_install_command \
	shellcheck \
	shellcheck \
	"missing shellcheck: install shellcheck to lint runtime toolchain scripts" \
	"just language/lint-toolchain" >&2 || exit 1

runtime_toolchain_shell_scripts=(
	"${script_directory}"/*.sh
	"${script_directory}"/lib/*.sh
)

for script_path in "${runtime_toolchain_shell_scripts[@]}"; do
	bash -n "${script_path}"
done

cd "${repository_root}"
shellcheck -x "${runtime_toolchain_shell_scripts[@]}"
