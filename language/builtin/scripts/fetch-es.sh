#!/usr/bin/env bash
# fetch typescript es libs into builtin/lib/es

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

TS_VERSION=${TS_VERSION:-latest}
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}

echo "  - es5"
mkdir -p "$SRC_DIR/es/es5"
curl -fsSL "$BASE_URL/lib.es5.d.ts" > "$SRC_DIR/es/es5/index.d.ds"

echo "  - es2015"
mkdir -p "$SRC_DIR/es/es2015"
for file in core collection generator iterable promise proxy reflect symbol symbol.wellknown; do
    curl -fsSL "$BASE_URL/lib.es2015.$file.d.ts" > "$SRC_DIR/es/es2015/$file.d.ds"
done

echo "  - es2016"
mkdir -p "$SRC_DIR/es/es2016"
curl -fsSL "$BASE_URL/lib.es2016.array.include.d.ts" > "$SRC_DIR/es/es2016/array.include.d.ds"
curl -fsSL "$BASE_URL/lib.es2016.intl.d.ts" > "$SRC_DIR/es/es2016/intl.d.ds"

echo "  - es2017"
mkdir -p "$SRC_DIR/es/es2017"
for file in date intl object sharedmemory string typedarrays; do
    curl -fsSL "$BASE_URL/lib.es2017.$file.d.ts" > "$SRC_DIR/es/es2017/$file.d.ds"
done

echo "  - es2018"
mkdir -p "$SRC_DIR/es/es2018"
for file in asyncgenerator asynciterable intl promise regexp; do
    curl -fsSL "$BASE_URL/lib.es2018.$file.d.ts" > "$SRC_DIR/es/es2018/$file.d.ds"
done

echo "  - es2019"
mkdir -p "$SRC_DIR/es/es2019"
for file in array intl object string symbol; do
    curl -fsSL "$BASE_URL/lib.es2019.$file.d.ts" > "$SRC_DIR/es/es2019/$file.d.ds"
done

echo "  - es2020"
mkdir -p "$SRC_DIR/es/es2020"
for file in bigint date intl number promise sharedmemory string symbol.wellknown; do
    curl -fsSL "$BASE_URL/lib.es2020.$file.d.ts" > "$SRC_DIR/es/es2020/$file.d.ds"
done

echo "  - es2021"
mkdir -p "$SRC_DIR/es/es2021"
for file in intl promise string weakref; do
    curl -fsSL "$BASE_URL/lib.es2021.$file.d.ts" > "$SRC_DIR/es/es2021/$file.d.ds"
done

echo "  - es2022"
mkdir -p "$SRC_DIR/es/es2022"
for file in array error intl object regexp sharedmemory string; do
    curl -fsSL "$BASE_URL/lib.es2022.$file.d.ts" > "$SRC_DIR/es/es2022/$file.d.ds"
done

echo "  - es2023"
mkdir -p "$SRC_DIR/es/es2023"
for file in array collection intl; do
    curl -fsSL "$BASE_URL/lib.es2023.$file.d.ts" > "$SRC_DIR/es/es2023/$file.d.ds"
done

echo "  - es2024"
mkdir -p "$SRC_DIR/es/es2024"
for file in arraybuffer collection object promise regexp sharedmemory string; do
    curl -fsSL "$BASE_URL/lib.es2024.$file.d.ts" > "$SRC_DIR/es/es2024/$file.d.ds"
done

echo "  - esnext"
mkdir -p "$SRC_DIR/es/esnext"
for file in array collection disposable intl iterator promise regexp string; do
    if ! curl -fsSL "$BASE_URL/lib.esnext.$file.d.ts" > "$SRC_DIR/es/esnext/$file.d.ds"; then
        echo "    missing esnext.$file"
    fi
done

echo "  - decorators"
curl -fsSL "$BASE_URL/lib.decorators.d.ts" > "$SRC_DIR/es/decorators.d.ds"
curl -fsSL "$BASE_URL/lib.decorators.legacy.d.ts" > "$SRC_DIR/es/decorators.legacy.d.ds"
