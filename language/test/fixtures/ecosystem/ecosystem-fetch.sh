#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

cargo test --release -p destack_test --test ecosystem -- --fetch "$@"

