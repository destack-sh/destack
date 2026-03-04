#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./scripts/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"
# shellcheck source=./scripts/toolchain/lib/runtime-windows-gnu.sh
source "${script_directory}/lib/runtime-windows-gnu.sh"

if [ "$#" -eq 0 ]; then
	echo "usage: runtime-windows-gnu-env.sh <command> [args...]"
	exit 1
fi

runtime_windows_gnu_apply_environment
exec "$@"
