#!/usr/bin/env bash
set -euo pipefail

mode="${1:-default}"

case "${mode}" in
default | full) ;;
*)
    echo "unsupported generate check mode: ${mode}"
    echo "supported: default, full"
    exit 1
    ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
language_root="${repo_root}/language"

before_diff="$(mktemp)"
after_diff="$(mktemp)"
before_untracked="$(mktemp)"
after_untracked="$(mktemp)"

cleanup() {
    rm -f "${before_diff}" "${after_diff}" "${before_untracked}" "${after_untracked}"
}

trap cleanup EXIT

capture_state() {
    local diff_path="$1"
    local untracked_path="$2"

    git -C "${repo_root}" diff --binary -- . > "${diff_path}"
    git -C "${repo_root}" ls-files --others --exclude-standard -- . | sort > "${untracked_path}"
}

run_default_generators() {
    cd "${language_root}"

    just generate-unicode
    just generate-lexer
    just generate-schema
    just generate-bindings
    just generate-builtin-libs
}

run_extended_generators() {
    cd "${language_root}"

    just generate-conformance-schema
    just update-conformance-catalog
}

run_full_generators() {
    run_default_generators
    run_extended_generators

    cd "${language_root}"
    just generate-stress
}

capture_state "${before_diff}" "${before_untracked}"

case "${mode}" in
default)
    run_default_generators
    ;;
full)
    run_full_generators
    ;;
esac

capture_state "${after_diff}" "${after_untracked}"

if ! cmp -s "${before_diff}" "${after_diff}" || ! cmp -s "${before_untracked}" "${after_untracked}"; then
    echo "generated files drifted after regeneration"
    echo "run the generator commands and commit the resulting updates"
    exit 1
fi

echo "generated files are up to date"
