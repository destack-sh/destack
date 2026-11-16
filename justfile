set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

# run every database test on SQLite and the test PostgreSQL
export DESTACK_TEST_POSTGRES := "postgres://postgres@127.0.0.1:55432/postgres"

# the CI gate runners: 54 gates paid 81 of 108 runner minutes in setup, 8 pay 12
runners := "8"

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

# run each package's gates: formatting, types, lints, and tests
gate +directories:
    #!/usr/bin/env bash
    set -euo pipefail
    # regenerate the generated settings locally, as CI checks them against the committed ones
    if [ -z "${CI:-}" ]; then
        for root in $(git ls-files '.oxfmtrc.json' '*/.oxfmtrc.json' | grep -v '/tests/' | xargs -n1 dirname); do
            (cd "{{justfile_directory()}}/$root" && bun run destack-check configure)
        done
    fi
    for directory in {{directories}}; do
        echo "gate $directory"
        just gate-check "$directory"
        just gate-test "$directory"
    done

# check a package's formatting, types and lints
gate-check directory:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}/{{directory}}"
    # regenerate the package's compiler settings locally, as CI checks them
    if [ -z "${CI:-}" ]; then
        bun run destack-check configure
    fi
    bun run destack-check format-check
    # typecheck through the package's script, which also compiles its README listings
    if [ -n "$(jq -r '.scripts.typecheck // ""' package.json)" ]; then
        bun run typecheck
    else
        bunx tsc --noEmit -p .
    fi
    bun run destack-check check

# test a package, or the given test files of it
gate-test directory *files:
    #!/usr/bin/env bash
    set -euo pipefail
    # give each test worker two cores, as a scenario test runs its workloads beside it
    cores="$(getconf _NPROCESSORS_ONLN)"
    workers=$(( cores / 2 > 4 ? 4 : (cores / 2 < 1 ? 1 : cores / 2) ))
    cd "{{justfile_directory()}}/{{directory}}"
    test="$(jq -r '.scripts.test // ""' package.json)"

    # run vitest with the files, other scripts as written
    if [[ "$test" == *"vitest run"* && "$test" != *"&&"* && "$test" != *";"* ]]; then
        bun run test --maxWorkers="$workers" {{files}}
    elif [ -n "{{files}}" ]; then
        echo "error: {{directory}} runs no single vitest to filter" >&2
        exit 1
    elif [[ "$test" == *vitest* ]]; then
        bun run test --maxWorkers="$workers"
    elif [ -n "$test" ]; then
        bun run test
    fi

# list a package's vitest test files
gate-files directory:
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
gate-durations directory milliseconds:
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
gate-runners durations +directories:
    #!/usr/bin/env bash
    set -euo pipefail
    # read the recorded durations, if any
    durations="{{durations}}"
    if [ ! -f "$durations" ]; then
        durations=/dev/null
    fi

    # list each package's test files
    for directory in {{directories}}; do
        just gate-files "$directory" | jq --raw-input --slurp --compact-output --arg package "$directory" \
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
    if [ "{{action}}" = up ]; then
        "$bin/pg_ctl" --pgdata "$data" status >/dev/null 2>&1 || just postgres start
        exit 0
    fi
    if [ "{{action}}" = start ] && [ ! -d "$data" ]; then
        "$bin/initdb" --pgdata "$data" --username postgres --auth trust --encoding UTF8 >/dev/null
    fi
    # run without durability, as the test cluster's data is disposable, with connections for a worker per core
    options="-p 55432 -k /tmp -c fsync=off -c synchronous_commit=off -c full_page_writes=off -c max_connections=300"
    "$bin/pg_ctl" --pgdata "$data" --log "$data/server.log" --options "$options" --wait "{{action}}"

# serve the local universe, a cloud machine per residency and the local stage's machine (local reset drops their state)
local *arguments: (postgres "up")
    just @platform/release/local {{arguments}}

# version
version:
    @bun --eval 'console.log(JSON.parse(await Bun.file("package.json").text()).version)'

# pack
pack target="":
    just @platform/release/pack "{{target}}"
