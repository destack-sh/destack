set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

# run every database test on SQLite and the test PostgreSQL
export DESTACK_TEST_POSTGRES := "postgres://postgres@127.0.0.1:55432/postgres"

# the CI check runners: 54 checks paid 81 of 108 runner minutes in setup, 8 pay 12
runners := "8"

_default:
    @just --list --unsorted

# install
install:
    bun install

# update
update:
    bun update

# build the site and the release
build:
    bun run --cwd @platform/site build
    bun run --cwd @platform/release build

# generate the site's pricing and headwords
generate:
    bun run --cwd @platform/site generate

# fix every package through its pinned toolchain: its generated settings, formatting and lints, the only recipe that writes
fix:
    #!/usr/bin/env bash
    set -euo pipefail
    # write the workspace root's settings, then fix each package in turn
    bun run destack-check configure
    cd "{{justfile_directory()}}/@destack/cli"
    for definition in "{{justfile_directory()}}"/@destack/*/destack.json "{{justfile_directory()}}"/@platform/*/destack.json; do
        bun run --silent dev fix "$(dirname "$definition")"
    done

alias fmt := fix

# check the workflows, the repository's documents and the shared settings, writing nothing
check-repository:
    actionlint
    bun run destack-check check README.md AGENTS.md CONTRIBUTING.md SECURITY.md @platform/site/content

# check the repository, then check and test each package the changes since a base affect, as CI does
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

# check the same packages with their unit tests alone, the loop while working
check-fast base="":
    #!/usr/bin/env bash
    set -euo pipefail
    just check-repository
    base="{{base}}"
    base="${base:-$(git merge-base HEAD origin/main)}"
    for directory in $(just affected "$base"); do
        echo "check $directory"
        # lint only the package's modules changed since the base, as the type checker checks only what it lints
        just check-package "$directory" --since "$base"
        just test "$directory" --fast
    done

# check and test every package in every tier, as a release does
check-all: (postgres "up")
    #!/usr/bin/env bash
    set -euo pipefail
    just check-repository
    for directory in $(just affected); do
        echo "check $directory"
        just check-package "$directory"
        just test "$directory"
    done

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

# check a package's formatting, types and lints, writing nothing: lint every source, or those `--since <base>` changed
check-package directory *arguments:
    #!/usr/bin/env bash
    set -euo pipefail
    # check the package through its pinned toolchain with the command line's sources
    cd "{{justfile_directory()}}/@destack/cli"
    bun run --silent dev check "{{justfile_directory()}}/{{directory}}" {{arguments}}

# test a package in every tier, or its unit tier with --fast, or the given test files of it
test directory *arguments:
    #!/usr/bin/env bash
    set -euo pipefail
    # read --fast as the unit tier alone, the other arguments as test files
    options=()
    files=()
    for argument in {{arguments}}; do
        if [ "$argument" = --fast ]; then
            options+=(--tier unit)
        else
            files+=("$argument")
        fi
    done
    # run the package's tests through its plan with the command line's sources
    cd "{{justfile_directory()}}/@destack/cli"
    bun run --silent dev test "{{justfile_directory()}}/{{directory}}" ${options[@]+"${options[@]}"} ${files[@]+"${files[@]}"}

# list a package's vitest test files
test-files directory:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}/{{directory}}"
    test="$(jq -r '.scripts.test // ""' package.json)"
    # list files with the test script's own options
    if [[ "$test" == *"vitest run"* && "$test" != *"&&"* && "$test" != *";"* ]]; then
        listed="$(mktemp)"
        PATH="$PWD/node_modules/.bin:{{justfile_directory()}}/node_modules/.bin:$PATH" \
            bash -c "${test/vitest run/vitest list --filesOnly --json=$listed}" >&2
        jq -r --arg root "$PWD/" '[.[].file | ltrimstr($root)] | unique[]' "$listed"
        rm "$listed"
    fi

# print each test file's share of a run's milliseconds as JSON
test-durations directory milliseconds:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}/{{directory}}"
    # share the run by each file's durations over its projects, from the results Vitest reads
    shopt -s nullglob
    jq --slurp --compact-output --arg package "{{directory}}" --argjson milliseconds "{{milliseconds}}" '
        [.[] | select(.version == 1) | .projects[] | .files // {} | to_entries[]]
        | group_by(.key)
        | map({ key: ($package + "/" + .[0].key), value: (map(.value.duration) | add) })
        | (map(.value) | add) as $sum
        | if $sum > 0 then map(.value = (.value * $milliseconds / $sum | round)) else [] end
        | from_entries
    ' node_modules/.vite/vitest/*/results.json /dev/null

# weigh a file by its duration, else its package's median, else the median, else 1
# keep a package whole, and cut a measured one heavier than the average runner into parts
# pack each package or part onto the lightest runner, heaviest first
# check a package on the runner with its largest share
# divide packages across the runners as JSON
check-runners durations +directories:
    #!/usr/bin/env bash
    set -euo pipefail
    # read the recorded durations, if any
    durations="{{durations}}"
    if [ ! -f "$durations" ]; then
        durations=/dev/null
    fi

    # list each package's test files
    for directory in {{directories}}; do
        just test-files "$directory" | jq --raw-input --slurp --compact-output --arg package "$directory" \
            '{ package: $package, files: (split("\n") | map(select(. != ""))) }'
    done | jq --slurp --compact-output --argjson runners "{{runners}}" --slurpfile table "$durations" '
        def median: sort | if length == 0 then null else .[length / 2 | floor] end;
        # pack items onto the lightest bin, heaviest first
        def pack($count): sort_by(-.weight, .key)
            | reduce .[] as $item ([range($count) | { weight: 0, items: [] }];
                (to_entries | min_by(.value.weight) | .key) as $lightest
                | .[$lightest].weight += $item.weight
                | .[$lightest].items += [$item]);

        ($table[0] // {}) as $table
        | ($table.files // {}) as $known
        | ([$known[]] | median // 1) as $fallback

        # weigh files, packages and checks
        | map(
            .package as $package
            | ([.files[] | $known[$package + "/" + .] | numbers] | median) as $local
            | .files |= map({ key: ., weight: ($known[$package + "/" + .] // $local // $fallback) })
            | .check = ($table.checks[$package] // 0)
            | .isMeasured = ($local != null)
            | .test = (if .files == [] then ($table.scripts[$package] // $fallback) else (.files | map(.weight) | add) end)
            | .weight = .check + .test
        )
        | (map({ key: .package, value: .check }) | from_entries) as $checks
        | ((map(.weight) | add) / $runners) as $average

        # cut measured packages heavier than the average into parts
        | [
            .[]
            | .package as $package
            | if .isMeasured and .weight > $average then
                .files
                | pack(map(.weight) | add / $average | ceil)
                | .[]
                | select(.items != [])
                | { key: ($package + "/" + .items[0].key), package: $package, files: [.items[].key], weight }
            else
                { key: $package, package: $package, files: [], weight }
            end
        ]

        # pack the runners, one run per package
        | pack($runners)
        | [
            to_entries[]
            | select(.value.items != [])
            | {
                runner: (.key + 1),
                milliseconds: .value.weight,
                work: (
                    .value.items
                    | group_by(.package)
                    | map({
                        package: .[0].package,
                        files: (if any(.files == []) then [] else [.[].files[]] | sort end),
                        share: (map(.weight) | add)
                    })
                )
            }
        ]

        # check each package on the runner with its largest share
        | (reduce (.[] | .runner as $runner | .work[] | { package, share, runner: $runner }) as $part ({};
            if (.[$part.package].share // -1) < $part.share then .[$part.package] = $part else . end)) as $owners
        | map(
            .runner as $runner
            | .work |= map(.isChecked = ($owners[.package].runner == $runner) | del(.share))
            | .milliseconds = (.milliseconds + ([.work[] | select(.isChecked and .files != []) | $checks[.package]] | add // 0) | round)
        )
    '

# start, stop or report the test PostgreSQL on port 55432, or start it unless it runs (up)
postgres action:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(brew --prefix postgresql@18)/bin"
    data=target/postgres
    # leave a server that already answers, such as CI's service, else start the local one
    if [ "{{action}}" = up ]; then
        pg_isready --host 127.0.0.1 --port 55432 >/dev/null 2>&1 || "$bin/pg_ctl" --pgdata "$data" status >/dev/null 2>&1 || just postgres start
        exit 0
    fi
    if [ "{{action}}" = start ] && [ ! -d "$data" ]; then
        "$bin/initdb" --pgdata "$data" --username postgres --auth trust --encoding UTF8 >/dev/null
    fi
    # run without durability, as the test cluster's data is disposable, with connections for a worker per core
    options="-p 55432 -k /tmp -c fsync=off -c synchronous_commit=off -c full_page_writes=off -c max_connections=300"
    "$bin/pg_ctl" --pgdata "$data" --log "$data/server.log" --options "$options" --wait "{{action}}"

# serve the local universe (local reset drops its state), and run the desktop app or the cli on it with local desktop and local cli
[no-exit-message]
[positional-arguments]
local *arguments:
    @case "${1:-serve}" in serve | reset) just postgres up ;; esac
    @bun run --cwd @platform/release local "$@"

# pack
pack target="":
    bun run --cwd @platform/release pack {{target}}

# serve the company site locally
company-dev:
    cd @platform/company && bunx wrangler@4.134.0 dev --port 3838

# deploy the company site with its token from .env.local
company-deploy:
    cd @platform/company && CLOUDFLARE_API_TOKEN="${CLOUDFLARE_COMPANY_API_TOKEN:?CLOUDFLARE_COMPANY_API_TOKEN is required}" bunx wrangler@4.134.0 deploy
