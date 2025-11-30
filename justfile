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
