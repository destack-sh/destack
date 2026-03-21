#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "$SCRIPT_DIR/prettier/fetch.sh"
bash "$SCRIPT_DIR/oxfmt/fetch.sh"
