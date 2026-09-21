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
    just app/build

# generate
generate:
    just platform/site/generate

# format
format:
    just library/format
    just platform/format
    just app/format
    bun run destack-check format service/*/src template/*/src

alias fmt := format

# check formatting
format-check:
    just library/format-check
    just platform/format-check
    bun run destack-check format-check app/*/src service/*/src template/*/src
    just dev/release/format-check

# lint
lint:
    just check-hygiene
    bun run destack-check check library/*/src service/*/src app/*/src platform/*/src template/*/src
    just platform/lint
    just app/check-quick

# test
test:
    just library/test
    bun run --cwd service/registry test
    bun run --cwd app/cli test
    just platform/test

# check
check:
    just check-hygiene
    just library/check
    bun run --cwd service/daemon check
    bun run --cwd service/registry check
    bun run --cwd template/blank check
    bun run --cwd template/stack check
    just platform/site/typecheck
    just platform/stack/check
    just app/check-quick

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
