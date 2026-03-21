#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "$SCRIPT_DIR/test262/fetch.sh"
bash "$SCRIPT_DIR/babel/fetch.sh"
bash "$SCRIPT_DIR/swc/fetch.sh"
bash "$SCRIPT_DIR/biome/fetch.sh"
