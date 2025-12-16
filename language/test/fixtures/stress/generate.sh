#!/usr/bin/env bash
# generate stress test fixtures
#
# usage:
#   ./generate.sh           # generate all fixtures
#   ./generate.sh parser
#   ./generate.sh resolver
#   ./generate.sh checker

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

generate_parser() {
    python3 "$SCRIPT_DIR/parser/generate.py"
}

generate_resolver() {
    python3 "$SCRIPT_DIR/resolver/generate.py"
}

generate_checker() {
    python3 "$SCRIPT_DIR/checker/generate.py"
}

generate_all() {
    generate_parser
    generate_resolver
    generate_checker
}

case "${1:-all}" in
    all)
        generate_all
        ;;
    parser)
        generate_parser
        ;;
    resolver)
        generate_resolver
        ;;
    checker)
        generate_checker
        ;;
    *)
        echo "unknown category: $1"
        echo "usage: $0 [all|parser|resolver|checker]"
        exit 1
        ;;
esac

echo "stress fixtures generated"
