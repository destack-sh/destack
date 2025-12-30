#!/usr/bin/env bash
# fetch typescript es libs into builtin/lib/es

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILTIN_DIR="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$BUILTIN_DIR/lib"

source "$SCRIPT_DIR/versions.sh"
BASE_URL=${BASE_URL:-"https://unpkg.com/typescript@$TS_VERSION/lib"}
source "$SCRIPT_DIR/fetch-util.sh"

echo "  - es5"
fetch_ts_lib "lib.es5.d.ts" "$SRC_DIR/es/es5/index.d.ds"

echo "  - es6"
fetch_ts_lib "lib.es6.d.ts" "$SRC_DIR/es/es6/index.d.ds"

echo "  - es2015"
fetch_ts_lib "lib.es2015.d.ts" "$SRC_DIR/es/es2015/index.d.ds"
for file in core collection generator iterable promise proxy reflect symbol symbol.wellknown; do
    fetch_ts_lib "lib.es2015.$file.d.ts" "$SRC_DIR/es/es2015/$file.d.ds"
done

echo "  - es2016"
fetch_ts_lib "lib.es2016.d.ts" "$SRC_DIR/es/es2016/index.d.ds"
fetch_ts_lib "lib.es2016.array.include.d.ts" "$SRC_DIR/es/es2016/array.include.d.ds"
fetch_ts_lib "lib.es2016.intl.d.ts" "$SRC_DIR/es/es2016/intl.d.ds"
fetch_ts_lib "lib.es2016.full.d.ts" "$SRC_DIR/es/es2016/full.d.ds"

echo "  - es2017"
fetch_ts_lib "lib.es2017.d.ts" "$SRC_DIR/es/es2017/index.d.ds"
for file in arraybuffer date intl object sharedmemory string typedarrays; do
    fetch_ts_lib "lib.es2017.$file.d.ts" "$SRC_DIR/es/es2017/$file.d.ds"
done
fetch_ts_lib "lib.es2017.full.d.ts" "$SRC_DIR/es/es2017/full.d.ds"

echo "  - es2018"
fetch_ts_lib "lib.es2018.d.ts" "$SRC_DIR/es/es2018/index.d.ds"
for file in asyncgenerator asynciterable intl promise regexp; do
    fetch_ts_lib "lib.es2018.$file.d.ts" "$SRC_DIR/es/es2018/$file.d.ds"
done
fetch_ts_lib "lib.es2018.full.d.ts" "$SRC_DIR/es/es2018/full.d.ds"

echo "  - es2019"
fetch_ts_lib "lib.es2019.d.ts" "$SRC_DIR/es/es2019/index.d.ds"
for file in array intl object string symbol; do
    fetch_ts_lib "lib.es2019.$file.d.ts" "$SRC_DIR/es/es2019/$file.d.ds"
done
fetch_ts_lib "lib.es2019.full.d.ts" "$SRC_DIR/es/es2019/full.d.ds"

echo "  - es2020"
fetch_ts_lib "lib.es2020.d.ts" "$SRC_DIR/es/es2020/index.d.ds"
for file in bigint date intl number promise sharedmemory string symbol.wellknown; do
    fetch_ts_lib "lib.es2020.$file.d.ts" "$SRC_DIR/es/es2020/$file.d.ds"
done
fetch_ts_lib "lib.es2020.full.d.ts" "$SRC_DIR/es/es2020/full.d.ds"

echo "  - es2021"
fetch_ts_lib "lib.es2021.d.ts" "$SRC_DIR/es/es2021/index.d.ds"
for file in intl promise string weakref; do
    fetch_ts_lib "lib.es2021.$file.d.ts" "$SRC_DIR/es/es2021/$file.d.ds"
done
fetch_ts_lib "lib.es2021.full.d.ts" "$SRC_DIR/es/es2021/full.d.ds"

echo "  - es2022"
fetch_ts_lib "lib.es2022.d.ts" "$SRC_DIR/es/es2022/index.d.ds"
for file in array error intl object regexp string; do
    fetch_ts_lib "lib.es2022.$file.d.ts" "$SRC_DIR/es/es2022/$file.d.ds"
done
fetch_ts_lib "lib.es2022.full.d.ts" "$SRC_DIR/es/es2022/full.d.ds"

echo "  - es2023"
fetch_ts_lib "lib.es2023.d.ts" "$SRC_DIR/es/es2023/index.d.ds"
for file in array collection intl; do
    fetch_ts_lib "lib.es2023.$file.d.ts" "$SRC_DIR/es/es2023/$file.d.ds"
done
fetch_ts_lib "lib.es2023.full.d.ts" "$SRC_DIR/es/es2023/full.d.ds"

echo "  - es2024"
fetch_ts_lib "lib.es2024.d.ts" "$SRC_DIR/es/es2024/index.d.ds"
for file in arraybuffer collection object promise regexp sharedmemory string; do
    fetch_ts_lib "lib.es2024.$file.d.ts" "$SRC_DIR/es/es2024/$file.d.ds"
done
fetch_ts_lib "lib.es2024.full.d.ts" "$SRC_DIR/es/es2024/full.d.ds"

echo "  - esnext"
fetch_ts_lib "lib.esnext.d.ts" "$SRC_DIR/es/esnext/index.d.ds"
for file in array collection decorators disposable error float16 intl iterator promise sharedmemory; do
    fetch_ts_lib "lib.esnext.$file.d.ts" "$SRC_DIR/es/esnext/$file.d.ds"
done
fetch_ts_lib "lib.esnext.full.d.ts" "$SRC_DIR/es/esnext/full.d.ds"

echo "  - decorators"
fetch_ts_lib "lib.decorators.d.ts" "$SRC_DIR/es/decorators.d.ds"
fetch_ts_lib "lib.decorators.legacy.d.ts" "$SRC_DIR/es/decorators.legacy.d.ds"
