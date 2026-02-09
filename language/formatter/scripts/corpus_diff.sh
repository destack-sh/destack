#!/usr/bin/env bash
set -euo pipefail

BASELINE_DIR="${1:-}"
CANDIDATE_DIR="${2:-}"

if [[ -z "$BASELINE_DIR" || -z "$CANDIDATE_DIR" ]]; then
    echo "usage: corpus_diff.sh <baseline_dir> <candidate_dir>"
    exit 2
fi

if [[ ! -d "$BASELINE_DIR" ]]; then
    echo "baseline directory does not exist: $BASELINE_DIR"
    exit 2
fi

if [[ ! -d "$CANDIDATE_DIR" ]]; then
    echo "candidate directory does not exist: $CANDIDATE_DIR"
    exit 2
fi

if diff -ru --strip-trailing-cr "$BASELINE_DIR" "$CANDIDATE_DIR"; then
    echo "no corpus drift detected"
else
    echo "corpus drift detected"
    exit 1
fi
