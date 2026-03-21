#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

bash "${script_directory}/check-runtime-android-rust.sh"
bash "${script_directory}/check-runtime-android-host.sh"
