set shell := ["bash", "-cu"]

_default:
    @just --list --unsorted

# install all dependencies
install:
    bun install
    just language/install-fixtures

# update all dependencies
update:
    bun update
    cargo update

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

# run all tests
test:
    just language/test
    just library/test
    just platform/test

# format all code
fmt:
    just language/fmt
    just library/fmt
    just platform/fmt

# clean all build artifacts
clean:
    just language/clean
    just library/clean

# bump version: major, minor, or patch
bump kind:
    cargo run --release -p destack_cli -- version {{ kind }}

# show current version
version:
    @cat version.txt

# publishable npm packages
npm_packages := "library/schema"

# publish all npm packages (dry-run by default)
publish-npm dry="--dry-run":
    just build
    for pkg in {{ npm_packages }}; do \
        echo "Publishing $pkg..."; \
        cd $pkg && bun publish {{ dry }} --access public && cd -; \
    done

# publish vscode extension to marketplace
publish-vscode:
    cd platform/vscode && bun run package
    cd platform/vscode && bunx --bun @vscode/vsce@latest publish

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
    echo "  just publish-npm ''"
    echo "  just publish-vscode"
