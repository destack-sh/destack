#!/usr/bin/env bash
# fetch builtin lib definitions into the builtin library
# run from the builtin directory: ./scripts/fetch.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"

source "$SCRIPT_DIR/versions.sh"
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}

export TS_VERSION
export BASE_URL

echo "fetching builtin lib files to $BUILTIN_DIR/lib"

"$SCRIPT_DIR/fetch-es.sh"
"$SCRIPT_DIR/fetch-dom.sh"
"$SCRIPT_DIR/fetch-worker.sh"
"$SCRIPT_DIR/fetch-scripthost.sh"
"$SCRIPT_DIR/fetch-node.sh"
"$SCRIPT_DIR/fetch-deno.sh"
"$SCRIPT_DIR/fetch-bun.sh"

echo "done! fetched from typescript $TS_VERSION"
