#!/usr/bin/env bash
set -euo pipefail

HTML5LIB_TESTS_VERSION="${HTML5LIB_TESTS_VERSION:-python-0.95-186}"
HTML5LIB_TESTS_REF="${HTML5LIB_TESTS_REF:-f994590f528ac8b6073665791ddb1ed85c66dfb2}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HTML5LIB_TARGET="$SCRIPT_DIR/html5lib"
HELPER="$SCRIPT_DIR/../conformance/fetch-fixtures.sh"

export REPO_URL="https://github.com/html5lib/html5lib-tests"

bash "$HELPER" "html5lib fixtures" "$HTML5LIB_TESTS_VERSION" "$HTML5LIB_TESTS_REF" "$HTML5LIB_TARGET" \
    "tokenizer" \
    "tree-construction" \
    "encoding"
