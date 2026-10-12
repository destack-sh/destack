set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

# run every database test on SQLite and the test PostgreSQL
export DESTACK_TEST_POSTGRES := "postgres://postgres@127.0.0.1:55432/postgres"

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
    just @platform/build

# generate
generate:
    just @platform/site/generate

# typecheck
typecheck:
    just @destack/typecheck
    just @app/typecheck
    just @template/typecheck
    just @platform/typecheck

# lint the workflows, every scope and the repository's documents
lint:
    actionlint
    just @destack/lint
    just @app/lint
    just @template/lint
    just @platform/lint
    bun run destack-check check README.md AGENTS.md CONTRIBUTING.md SECURITY.md @platform/site/content

# check formatting
format-check:
    just @destack/format-check
    just @app/format-check
    just @template/format-check
    just @platform/format-check

# typecheck, lint and check formatting
check: typecheck lint format-check

# test
test: (postgres "up")
    just @destack/test
    just @app/test
    just @platform/test

# format
format:
    just @destack/format
    just @app/format
    just @template/format
    just @platform/format

alias fmt := format

# test the packages the changes since a base commit affect, with their dependents
test-affected base="origin/main": (postgres "up")
    #!/usr/bin/env bash
    set -euo pipefail
    directories="$(bun run destack-check affected "{{base}}")"
    # run everything when a change reaches every package
    if [ "$directories" = "*" ]; then
        just test
        exit 0
    fi
    filters=()
    for directory in $directories; do
        filters+=(--filter "./$directory")
    done
    if [ ${#filters[@]} -eq 0 ]; then
        echo "no package is affected"
        exit 0
    fi
    bun run --bun --sequential --no-exit-on-error "${filters[@]}" --if-present test

# start, stop or report the test PostgreSQL on port 55432, or start it unless it runs (up)
postgres action:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(brew --prefix postgresql@18)/bin"
    data=target/postgres
    if [ "{{action}}" = up ]; then
        "$bin/pg_ctl" --pgdata "$data" status >/dev/null 2>&1 || just postgres start
        exit 0
    fi
    if [ "{{action}}" = start ] && [ ! -d "$data" ]; then
        "$bin/initdb" --pgdata "$data" --username postgres --auth trust --encoding UTF8 >/dev/null
    fi
    "$bin/pg_ctl" --pgdata "$data" --log "$data/server.log" --options "-p 55432 -k /tmp" --wait "{{action}}"

# serve the local universe, a cloud machine per residency and the dev channel's machine (dev reset drops their state)
dev *arguments: (postgres "up")
    just @platform/stack/dev {{arguments}}

# version
version:
    @bun --eval 'console.log(JSON.parse(await Bun.file("package.json").text()).version)'

# pack
pack target="":
    just @platform/release/pack "{{target}}"
