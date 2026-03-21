#!/usr/bin/env bash
set -euo pipefail

NODE_REF="${NODE_REF:-7547e795ef700e1808702fc2851a0dcc3395a065}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/checkouts/node"
HELPER="$SCRIPT_DIR/../fetch-fixtures.sh"

mkdir -p "$SCRIPT_DIR/checkouts"

export REPO_URL="https://github.com/nodejs/node.git"
bash "$HELPER" "node core fixtures" "main" "$NODE_REF" "$TARGET_DIR" \
    "test"
