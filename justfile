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
    just @platform/typecheck

# lint the workflows, every scope and the repository's documents
lint:
    actionlint
    just @destack/lint
    just @platform/lint
    bun run destack-check check README.md AGENTS.md CONTRIBUTING.md SECURITY.md @platform/site/content

# check formatting
format-check:
    just @destack/format-check
    just @platform/format-check

# typecheck, lint and check formatting
check: typecheck lint format-check

# test
test: (postgres "up")
    just @destack/test
    just @platform/test

# regenerate the lint and format settings, then format every package
format:
    #!/usr/bin/env bash
    set -euo pipefail
    # regenerate each tool root's settings: the workspace's and each template's, never a test's byte-exact fixtures
    for directory in $(git ls-files '.oxfmtrc.json' '*/.oxfmtrc.json' | grep -v '/tests/' | xargs -n1 dirname); do
        (cd "$directory" && bun run destack-check configure)
    done
    just @destack/format
    just @platform/format

alias fmt := format

# list the packages the changes since a base commit affect with their dependents, or every package
affected base="":
    #!/usr/bin/env bash
    set -euo pipefail
    directories="*"
    if [ -n "{{base}}" ]; then
        directories="$(bun run destack-check affected "{{base}}")"
    fi
    # list every package when a change reaches every package
    if [ "$directories" = "*" ]; then
        directories="$(printf '%s\n' @destack/* @platform/*)"
    fi
    # keep packages with sources, leaving out test fixtures
    for directory in $directories; do
        if [ -f "$directory/package.json" ] && [ -d "$directory/src" ] && [[ "$directory" != */tests/* ]]; then
            echo "$directory"
        fi
    done

# run each package's gates: formatting, types, lints, and tests on a worker per two cores, four at most
gate +directories:
    #!/usr/bin/env bash
    set -euo pipefail
    # give each test worker two cores, as a scenario test runs its workloads beside it
    cores="$(getconf _NPROCESSORS_ONLN)"
    workers=$(( cores / 2 > 4 ? 4 : (cores / 2 < 1 ? 1 : cores / 2) ))
    for directory in {{directories}}; do
        echo "gate $directory"
        cd "{{justfile_directory()}}/$directory"
        bun run destack-check format-check
        bunx tsc --noEmit -p .
        bun run destack-check check
        # run vitest suites on the workers, and other test scripts as written
        test="$(jq -r '.scripts.test // ""' package.json)"
        if [[ "$test" == *vitest* ]]; then
            bun run test --maxWorkers="$workers"
        elif [ -n "$test" ]; then
            bun run test
        fi
    done

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

# serve the local universe, a cloud machine per residency and the local stage's machine (local reset drops their state)
local *arguments: (postgres "up")
    just @platform/release/local {{arguments}}

# version
version:
    @bun --eval 'console.log(JSON.parse(await Bun.file("package.json").text()).version)'

# pack
pack target="":
    just @platform/release/pack "{{target}}"
