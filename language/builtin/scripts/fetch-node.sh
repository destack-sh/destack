#!/usr/bin/env bash
# fetch node types into builtin/lib/node

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/node"

NODE_TYPES_VERSION=${NODE_TYPES_VERSION:-"20.11.30"}
NODE_TYPES_BASE_URL=${NODE_TYPES_BASE_URL:-${NODE_TYPES_URL:-"https://unpkg.com/@types/node@$NODE_TYPES_VERSION"}}
NODE_BASE_URL=$NODE_TYPES_BASE_URL

export NODE_BASE_URL
export NODE_TYPES_VERSION

mkdir -p "$SRC_DIR"

echo "  - node @types/node $NODE_TYPES_VERSION"

NODE_DEST="$SRC_DIR/index.d.ds" python3 - <<'PY'
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
echo "done"
