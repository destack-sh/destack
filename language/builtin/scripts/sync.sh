#!/bin/bash
# Sync TypeScript lib definitions into the builtin library
# Run from the builtin directory: ./scripts/sync-libs.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/src/lib"

TS_VERSION=${TS_VERSION:-latest}
BASE_URL="https://unpkg.com/typescript@$TS_VERSION/lib"

echo "Syncing TypeScript $TS_VERSION lib files to $SRC_DIR"

# ES5
echo "  - es5"
mkdir -p "$SRC_DIR/es/es5"
curl -sL "$BASE_URL/lib.es5.d.ts" > "$SRC_DIR/es/es5/index.d.ds"

# ES2015
echo "  - es2015"
mkdir -p "$SRC_DIR/es/es2015"
for file in core collection generator iterable promise proxy reflect symbol symbol.wellknown; do
    curl -sL "$BASE_URL/lib.es2015.$file.d.ts" > "$SRC_DIR/es/es2015/$file.d.ds"
done

# ES2016
echo "  - es2016"
mkdir -p "$SRC_DIR/es/es2016"
curl -sL "$BASE_URL/lib.es2016.array.include.d.ts" > "$SRC_DIR/es/es2016/array.include.d.ds"
curl -sL "$BASE_URL/lib.es2016.intl.d.ts" > "$SRC_DIR/es/es2016/intl.d.ds"

# ES2017
echo "  - es2017"
mkdir -p "$SRC_DIR/es/es2017"
for file in date intl object sharedmemory string typedarrays; do
    curl -sL "$BASE_URL/lib.es2017.$file.d.ts" > "$SRC_DIR/es/es2017/$file.d.ds"
done

# ES2018
echo "  - es2018"
mkdir -p "$SRC_DIR/es/es2018"
for file in asyncgenerator asynciterable intl promise regexp; do
    curl -sL "$BASE_URL/lib.es2018.$file.d.ts" > "$SRC_DIR/es/es2018/$file.d.ds"
done

# ES2019
echo "  - es2019"
mkdir -p "$SRC_DIR/es/es2019"
for file in array intl object string symbol; do
    curl -sL "$BASE_URL/lib.es2019.$file.d.ts" > "$SRC_DIR/es/es2019/$file.d.ds"
done

# ES2020
echo "  - es2020"
mkdir -p "$SRC_DIR/es/es2020"
for file in bigint date intl number promise sharedmemory string symbol.wellknown; do
    curl -sL "$BASE_URL/lib.es2020.$file.d.ts" > "$SRC_DIR/es/es2020/$file.d.ds"
done

# ES2021
echo "  - es2021"
mkdir -p "$SRC_DIR/es/es2021"
for file in intl promise string weakref; do
    curl -sL "$BASE_URL/lib.es2021.$file.d.ts" > "$SRC_DIR/es/es2021/$file.d.ds"
done

# ES2022
echo "  - es2022"
mkdir -p "$SRC_DIR/es/es2022"
for file in array error intl object regexp sharedmemory string; do
    curl -sL "$BASE_URL/lib.es2022.$file.d.ts" > "$SRC_DIR/es/es2022/$file.d.ds"
done

# ES2023
echo "  - es2023"
mkdir -p "$SRC_DIR/es/es2023"
for file in array collection intl; do
    curl -sL "$BASE_URL/lib.es2023.$file.d.ts" > "$SRC_DIR/es/es2023/$file.d.ds"
done

# ES2024
echo "  - es2024"
mkdir -p "$SRC_DIR/es/es2024"
for file in arraybuffer collection object promise regexp sharedmemory string; do
    curl -sL "$BASE_URL/lib.es2024.$file.d.ts" > "$SRC_DIR/es/es2024/$file.d.ds"
done

# ESNext
echo "  - esnext"
mkdir -p "$SRC_DIR/es/esnext"
for file in array collection disposable intl iterator promise; do
    curl -sL "$BASE_URL/lib.esnext.$file.d.ts" > "$SRC_DIR/es/esnext/$file.d.ds" 2>/dev/null || true
done

# Decorators
echo "  - decorators"
curl -sL "$BASE_URL/lib.decorators.d.ts" > "$SRC_DIR/es/decorators.d.ds"
curl -sL "$BASE_URL/lib.decorators.legacy.d.ts" > "$SRC_DIR/es/decorators.legacy.d.ds"

# DOM
echo "  - dom"
mkdir -p "$SRC_DIR/dom"
curl -sL "$BASE_URL/lib.dom.d.ts" > "$SRC_DIR/dom/index.d.ds"
curl -sL "$BASE_URL/lib.dom.iterable.d.ts" > "$SRC_DIR/dom/iterable.d.ds"
curl -sL "$BASE_URL/lib.dom.asynciterable.d.ts" > "$SRC_DIR/dom/asynciterable.d.ds"

# WebWorker
echo "  - worker"
mkdir -p "$SRC_DIR/worker"
curl -sL "$BASE_URL/lib.webworker.d.ts" > "$SRC_DIR/worker/index.d.ds"
curl -sL "$BASE_URL/lib.webworker.iterable.d.ts" > "$SRC_DIR/worker/iterable.d.ds"
curl -sL "$BASE_URL/lib.webworker.asynciterable.d.ts" > "$SRC_DIR/worker/asynciterable.d.ds"

echo "Done! Synced from TypeScript $TS_VERSION"
