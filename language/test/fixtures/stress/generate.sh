#!/usr/bin/env bash
# generate stress test fixtures
#
# usage:
#   ./generate.sh           # generate all fixtures
#   ./generate.sh large_files
#   ./generate.sh large_projects
#   ./generate.sh concurrent
#   ./generate.sh memory

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

generate_large_files() {
    python3 "$SCRIPT_DIR/large_files/generate.py"
}

generate_large_projects() {
    python3 "$SCRIPT_DIR/large_projects/generate.py"
}

generate_memory() {
    python3 "$SCRIPT_DIR/memory/generate.py"
}

generate_edge_cases() {
    python3 "$SCRIPT_DIR/edge_cases/generate.py"
}

generate_pathological() {
    python3 "$SCRIPT_DIR/pathological/generate.py"
}

generate_all() {
    generate_large_files
    generate_large_projects
    generate_memory
    generate_edge_cases
    generate_pathological
}

case "${1:-all}" in
    all)
        generate_all
        ;;
    large_files)
        generate_large_files
        ;;
    large_projects)
        generate_large_projects
        ;;
    memory)
        generate_memory
        ;;
    edge_cases)
        generate_edge_cases
        ;;
    pathological)
        generate_pathological
        ;;
    *)
        echo "unknown category: $1"
        echo "usage: $0 [all|large_files|large_projects|memory|edge_cases|pathological]"
        exit 1
        ;;
esac

echo "stress fixtures generated"
