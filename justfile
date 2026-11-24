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

# build the site and the distribution
build:
    bun run --cwd @platform/site build
    bun run --cwd @platform/distribution build

# generate the site's pricing and headwords
generate:
    bun run --cwd @platform/site generate

# fix every package's settings, formatting and lints
fix:
    #!/usr/bin/env bash
    set -euo pipefail
    # write the root's settings, then fix each package
    bun run destack-check configure
    cd "{{justfile_directory()}}/@destack/cli"
    for definition in "{{justfile_directory()}}"/@destack/*/destack.json "{{justfile_directory()}}"/@platform/*/destack.json; do
        bun run --silent dev fix "$(dirname "$definition")"
    done

alias fmt := fix

# check the workflows, documents, shared settings and the company site's format
check-repository:
    actionlint
    bun run destack-check check README.md AGENTS.md ARCHITECTURE.md CONTRIBUTING.md SECURITY.md @platform/site/content
    bun run destack-check format-check @platform/company/public

# check and test the packages changed since a base, as CI does
check base="": (postgres "up")
    #!/usr/bin/env bash
    set -euo pipefail
    just check-repository
    base="{{base}}"
    for directory in $(just affected "${base:-$(git merge-base HEAD origin/main)}"); do
        echo "check $directory"
        just check-package "$directory"
        just test "$directory"
    done

# check the changed packages with their unit tests alone
check-fast base="":
    #!/usr/bin/env bash
    set -euo pipefail
    just check-repository
    base="{{base}}"
    base="${base:-$(git merge-base HEAD origin/main)}"
    for directory in $(just affected "$base"); do
        echo "check $directory"
        # lint only the modules changed since the base
        just check-package "$directory" --since "$base"
        just test "$directory" --fast
    done

# check and test every package in every tier
check-all: (postgres "up")
    #!/usr/bin/env bash
    set -euo pipefail
    just check-repository
    for directory in $(just affected); do
        echo "check $directory"
        just check-package "$directory"
        just test "$directory"
    done

# list the packages changed since a base and their dependents, else every package
affected base="":
    #!/usr/bin/env bash
    set -euo pipefail
    directories="*"
    if [ -n "{{base}}" ]; then
        directories="$(bun run destack-check affected "{{base}}")"
    fi
    # list every package for a change reaching all of them
    if [ "$directories" = "*" ]; then
        directories="$(printf '%s\n' @destack/* @platform/*)"
    fi
    # keep packages with sources, without test fixtures
    for directory in $directories; do
        if [ -f "$directory/package.json" ] && [ -d "$directory/src" ] && [[ "$directory" != */tests/* ]]; then
            echo "$directory"
        fi
    done

# check a package's formatting, types and lints
check-package directory *arguments:
    #!/usr/bin/env bash
    set -euo pipefail
    # check the package with its pinned toolchain
    cd "{{justfile_directory()}}/@destack/cli"
    bun run --silent dev check "{{justfile_directory()}}/{{directory}}" {{arguments}}

# test a package, its unit tier with --fast, its bench tier with --bench, or given files
test directory *arguments:
    #!/usr/bin/env bash
    set -euo pipefail
    # read the tier flags and the test files
    options=()
    files=()
    for argument in {{arguments}}; do
        if [ "$argument" = --fast ]; then
            options+=(--tier unit)
        elif [ "$argument" = --bench ]; then
            options+=(--tier bench)
        else
            files+=("$argument")
        fi
    done
    # run the package's tests through its plan
    cd "{{justfile_directory()}}/@destack/cli"
    bun run --silent dev test "{{justfile_directory()}}/{{directory}}" ${options[@]+"${options[@]}"} ${files[@]+"${files[@]}"}

# bench every package's scenarios --count times in one run, then print the medians
bench *arguments:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}"
    # read the run count, as Go's -count, and the named packages
    count=1
    packages=""
    set -- {{arguments}}
    while [ $# -gt 0 ]; do
        if [ "$1" = --count ]; then
            count="$2"
            shift 2
        elif [ "$1" = --cpuprofile ]; then
            export DESTACK_BENCH_CPUPROFILE=1
            shift
        else
            packages="$packages $1"
            shift
        fi
    done
    # select the named packages, else every package with scenarios
    if [ -z "$packages" ]; then
        packages="$(grep -rl --include='*.scenario.ts' -e defineScenario @destack @platform | grep -v node_modules | cut -d/ -f1,2 | sort -u)"
    fi
    # name the run after the commit
    baseline="$(git rev-parse --short HEAD)"
    if [ -n "$(git status --porcelain)" ]; then
        baseline="$baseline-dirty"
    fi
    # clear and bench each package
    for directory in $packages; do
        echo "bench $directory"
        name="$(jq --raw-output .name "$directory/package.json")"
        rm -rf "target/bench/$baseline/$name"
        DESTACK_BENCH_COUNT="$count" just test "$directory" --bench
    done
    bun run @destack/test/src/bench/main.ts target/bench "$baseline"

# print the CI runners for packages, as JSON: their checks and test files not yet passed, packed by duration
schedule +directories:
    @cd "{{justfile_directory()}}/@destack/check" && bun run --silent destack-check schedule {{directories}}

# start, stop or report the test PostgreSQL
postgres action:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(brew --prefix postgresql@18)/bin"
    data=target/postgres
    # reuse a running server, else start the local one
    if [ "{{action}}" = up ]; then
        pg_isready --host 127.0.0.1 --port 55432 >/dev/null 2>&1 || "$bin/pg_ctl" --pgdata "$data" status >/dev/null 2>&1 || just postgres start
        exit 0
    fi
    if [ "{{action}}" = start ] && [ ! -d "$data" ]; then
        "$bin/initdb" --pgdata "$data" --username postgres --auth trust --encoding UTF8 >/dev/null
    fi
    # run without durability, with a connection per worker
    options="-p 55432 -k /tmp -c fsync=off -c synchronous_commit=off -c full_page_writes=off -c max_connections=300"
    "$bin/pg_ctl" --pgdata "$data" --log "$data/server.log" --options "$options" --wait "{{action}}"

# run the local universe: serve, reset, publish, desktop or cli
[no-exit-message]
[positional-arguments]
local *arguments:
    @case "${1:-serve}" in serve | reset) just postgres up ;; esac
    @bun run --cwd @platform/deployment local "$@"

# pack
pack target="":
    bun run --cwd @platform/distribution pack {{target}}

# serve the company site locally
company-dev:
    cd @platform/company && bunx wrangler@4.134.0 dev --port 3838

# deploy the company site
company-deploy:
    cd @platform/company && CLOUDFLARE_API_TOKEN="${CLOUDFLARE_COMPANY_API_TOKEN:?CLOUDFLARE_COMPANY_API_TOKEN is required}" bunx wrangler@4.134.0 deploy
