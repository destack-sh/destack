#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

bash "${script_directory}/check-runtime-ios-rust.sh"
bash "${script_directory}/check-runtime-ios-host.sh"
