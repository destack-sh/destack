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

# build everything
build:
    just language/build
    just library/napi
    just platform/build

# build napi bindings
build-napi:
    just library/napi

# build CLI
build-cli:
    just platform/cli

# build LSP
build-lsp:
    just platform/lsp

# build VSCode extension
build-vscode:
    just platform/vscode

# run all tests
test:
    just language/test
    just library/test
    just platform/test

# format all code
fmt:
    just language/fmt
    just library/fmt

# check formatting
fmt-check:
    just language/fmt-check
    just library/fmt-check

# lint all code
lint:
    just language/lint
    just library/lint
    just platform/lint

# clean all build artifacts
clean:
    just language/clean
    just library/clean
