set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

_default:
    @just --list --unsorted

# install
install:
    bun install

# update
update:
    bun update

# build
build:
    just platform/build
    just dev/release/build

# generate
generate:
    just platform/site/generate

# format
format:
    just @destack/format
    just platform/format
    just dev/release/format

alias fmt := format

# check formatting
format-check:
    just @destack/format-check
    just platform/format-check
    just dev/release/format-check

# lint
lint:
    just check-hygiene
    bun run destack-check check @destack/*/src platform/*/src
    bun run destack-check check README.md AGENTS.md CONTRIBUTING.md SECURITY.md docs blog @destack/*/README.md
    just platform/lint
    just dev/release/check

# test
test:
    just @destack/test
    just platform/test

# check
check:
    just check-hygiene
    just @destack/check
    just platform/site/typecheck
    just platform/stack/check
    just dev/release/check

alias check-quick := check
alias check-full := check

# check hygiene
check-hygiene:
    actionlint

# version
version:
    @bun --eval 'console.log(JSON.parse(await Bun.file("package.json").text()).version)'

# pack
pack target="":
    just dev/release/pack "{{target}}"
