#!/usr/bin/env bash
# fetch deno types into builtin/lib/deno

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/deno"

DENO_VERSION=${DENO_VERSION:-"1.45.0"}
DENO_TYPES_BASE_URL=${DENO_TYPES_BASE_URL:-${DENO_TYPES_URL:-"https://raw.githubusercontent.com/denoland/deno/v$DENO_VERSION/cli/tsc/dts"}}
DENO_BASE_URL=$DENO_TYPES_BASE_URL

export DENO_BASE_URL
export DENO_VERSION

mkdir -p "$SRC_DIR"

echo "  - deno $DENO_VERSION"

DENO_DEST="$SRC_DIR/index.d.ds" python3 - <<'PY'
import os
import re
import urllib.request
from pathlib import Path

base_url = os.environ["DENO_BASE_URL"].rstrip("/")
dest_path = Path(os.environ["DENO_DEST"])

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

fetch("lib.deno.d.ts")

dest_path.write_text("\n\n".join(section for section in output if section) + "\n")
PY

echo "done"
