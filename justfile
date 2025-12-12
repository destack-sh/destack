set shell := ["bash", "-cu"]

_default:
    @just --list --unsorted

# --- setup ---

# install all dependencies
install:
    bun install
    just language/install

# update all dependencies
update:
    bun update
    cargo update

# --- build ---

# check everything
check:
    just language/check
    just library/check
    just platform/check

# build everything
build:
    just language/build
    just library/napi
    just platform/build

# format all code
format:
    just language/format
    just library/format
    just platform/format

# alias for format
fmt := "format"

# clean all build artifacts
clean:
    just language/clean
    just library/clean
    just platform/clean

# --- test ---

# run all tests
test:
    just language/test
    just library/test
    just platform/test

# --- bench ---

# run benchmarks
bench:
    just language/bench

# --- fuzz ---

# run fuzzers
fuzz duration="60":
    just language/fuzz {{duration}}

# --- release ---

# show current version
version:
    @cat version.txt

# bump version: major, minor, or patch
bump kind:
    cargo run --release -p destack_cli -- version {{ kind }}

# publish all packages (dry-run by default)
publish dry="--dry-run":
    just build
    just library/publish {{dry}}
    just platform/publish {{dry}}

# create a new release (bump, commit, tag, push)
release kind message:
    #!/usr/bin/env bash
    set -euo pipefail

    # bump version
    just bump {{ kind }}
    VERSION=$(cat version.txt)

    # check if changelog has entry for this version
    if ! grep -q "## \[${VERSION}\]" CHANGELOG.md; then
        echo "Warning: No changelog entry found for version ${VERSION}"
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Aborting release."
            exit 1
        fi
    fi

    # stage and commit
    git add -A
    git commit -m "{{ message }}"

    # create tag
    git tag -a "v${VERSION}" -m "Release v${VERSION}"

    echo ""
    echo "Release v${VERSION} created locally."
    echo "To publish:"
    echo "  git push origin main --tags"
    echo "  just publish ''"
