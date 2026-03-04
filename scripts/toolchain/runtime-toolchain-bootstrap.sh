#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
"${script_directory}/runtime-toolchain-install.sh"
