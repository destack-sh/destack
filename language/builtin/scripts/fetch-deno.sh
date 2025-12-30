#!/usr/bin/env bash
# fetch deno types into builtin/lib/deno

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib/deno"
source "$SCRIPT_DIR/fetch-util.sh"

DENO_TARGETS_DEFAULT=(
    "2.6:2.6.3"
)

DENO_TARGETS=("${DENO_TARGETS_DEFAULT[@]}")
if [[ -n "${DENO_TARGETS_OVERRIDE:-}" ]]; then
    IFS=',' read -r -a DENO_TARGETS <<< "$DENO_TARGETS_OVERRIDE"
fi

for target in "${DENO_TARGETS[@]}"; do
    IFS=':' read -r deno_lib_version deno_version <<< "$target"

    if [[ -z "$deno_lib_version" || -z "$deno_version" ]]; then
        fail "invalid deno target $target"
    fi

    DENO_BASE_URL=${DENO_TYPES_BASE_URL:-${DENO_TYPES_URL:-"https://raw.githubusercontent.com/denoland/deno/v$deno_version/cli/tsc/dts"}}

    DENO_LIB_DIR="$SRC_DIR/v$deno_lib_version"
    mkdir -p "$DENO_LIB_DIR"

    echo "  - deno.v$deno_lib_version $deno_version"

    if ! DENO_BASE_URL="$DENO_BASE_URL" DENO_DEST="$DENO_LIB_DIR/index.d.ds" python3 - <<'PY'
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

fetch("lib.deno.ns.d.ts")

dest_path.write_text("\n\n".join(section for section in output if section) + "\n")
PY
    then
        fail "missing deno types from $DENO_BASE_URL"
    fi

    echo "done"
done
