#!/usr/bin/env bash
# fetch node types into builtin/lib/node

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/node"
source "$SCRIPT_DIR/fetch-util.sh"

NODE_TARGETS_DEFAULT=(
    "18:18.19.130"
    "20:20.19.27"
    "22:22.19.3"
    "24:24.10.4"
)

NODE_TARGETS=("${NODE_TARGETS_DEFAULT[@]}")
if [[ -n "${NODE_TARGETS_OVERRIDE:-}" ]]; then
    IFS=',' read -r -a NODE_TARGETS <<< "$NODE_TARGETS_OVERRIDE"
fi

for target in "${NODE_TARGETS[@]}"; do
    IFS=':' read -r node_lib_version node_types_version <<< "$target"

    if [[ -z "$node_lib_version" || -z "$node_types_version" ]]; then
        fail "invalid node target $target"
    fi

    NODE_LIB_DIR="$SRC_DIR/v$node_lib_version"
    mkdir -p "$NODE_LIB_DIR"

    NODE_BASE_URL=${NODE_TYPES_BASE_URL:-${NODE_TYPES_URL:-"https://unpkg.com/@types/node@$node_types_version"}}

    echo "  - node.v$node_lib_version @types/node $node_types_version"

    if ! NODE_BASE_URL="$NODE_BASE_URL" NODE_DEST="$NODE_LIB_DIR/index.d.ds" python3 - <<'PY'
import os
import re
import urllib.request
from pathlib import Path

base_url = os.environ["NODE_BASE_URL"].rstrip("/")
dest_path = Path(os.environ["NODE_DEST"])

seen = set()
output = []

def fetch(path: str) -> None:
    if path in seen:
        return
    seen.add(path)

    url = f"{base_url}/{path}"
    with urllib.request.urlopen(url) as resp:
        data = resp.read().decode("utf-8")

    references = re.findall(r'^///\\s*<reference\\s+path="([^"]+)"\\s*/>\\s*$', data, flags=re.M)
    for ref in references:
        fetch(ref)

    data = re.sub(r'^///\\s*<reference\\s+path="[^"]+"\\s*/>\\s*$', "", data, flags=re.M)
    output.append(data.strip())

fetch("index.d.ts")

dest_path.write_text("\n\n".join(section for section in output if section) + "\n")
PY
    then
        fail "missing node types from $NODE_BASE_URL"
    fi

    echo "done"
done
