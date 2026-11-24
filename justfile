set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

# run every database test on SQLite and the test PostgreSQL
export DESTACK_TEST_POSTGRES := "postgres://postgres@127.0.0.1:55432/postgres"

# the milliseconds of work each CI runner takes: 2 to 3 minutes of wall time minus about a minute of setup
runner_milliseconds := "120000"

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

# list a package's vitest test files
test-files directory:
    #!/usr/bin/env bash
    set -euo pipefail
    # list the files the test plan runs in the unit and scenario tiers, each once
    cd "{{justfile_directory()}}/@destack/test"
    bun --eval '
        import { TestPlan } from "@destack/test/plan";
        import { BunEnvironment } from "@destack/environment/bun";
        const plan = await TestPlan.of(process.argv[1], BunEnvironment.system.files);
        const files = plan.targets.filter((target) => target.tier !== "bench").flatMap((target) => target.files);
        console.log([...new Set(files)].sort().join("\n"));
    ' "{{justfile_directory()}}/{{directory}}"

# print each test file's share of a run's milliseconds as JSON
test-durations directory milliseconds:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}/{{directory}}"
    # sum each file's durations over its projects
    shopt -s nullglob
    jq --slurp --compact-output --arg package "{{directory}}" --argjson milliseconds "{{milliseconds}}" '
        [.[] | select(.version == 1) | .projects[] | .files // {} | to_entries[]]
        | group_by(.key)
        | map({ key: ($package + "/" + .[0].key), value: (map(.value.duration) | add) })
        | (map(.value) | add) as $sum
        | if $sum > 0 then map(.value = (.value * $milliseconds / $sum | round)) else [] end
        | from_entries
    ' node_modules/.vite/vitest/*/results.json /dev/null

# divide packages across the runners by duration, as JSON
check-runners durations +directories:
    #!/usr/bin/env bash
    set -euo pipefail
    # read the recorded durations
    durations="{{durations}}"
    if [ ! -f "$durations" ]; then
        durations=/dev/null
    fi

    # list each package's test files
    for directory in {{directories}}; do
        just test-files "$directory" | jq --raw-input --slurp --compact-output --arg package "$directory" \
            '{ package: $package, files: (split("\n") | map(select(. != ""))) }'
    done | jq --slurp --compact-output --argjson target "{{runner_milliseconds}}" --slurpfile table "$durations" '
        def median: sort | if length == 0 then null else .[length / 2 | floor] end;
        # pack items onto the lightest bin
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
            | .test = (if .files == [] then ($table.scripts[$package] // $fallback) else (.files | map(.weight) | add) end)
            | .weight = .check + .test
        )
        | (map({ key: .package, value: .check }) | from_entries) as $checks
        | ([((map(.weight) | add) / $target | ceil), 1] | max) as $runners
        | ((map(.weight) | add) / $runners) as $average

        # split each package heavier than the average runner by its files
        | [
            .[]
            | .package as $package
            | if .files != [] and .weight > $average then
                .files
                | pack(map(.weight) | add / $average | ceil)
                | .[]
                | select(.items != [])
                | { key: ($package + "/" + .items[0].key), package: $package, files: [.items[].key], weight }
            else
                { key: $package, package: $package, files: [], weight }
            end
        ]

        # pack the runners
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

        # check each package on its largest runner
        | (reduce (.[] | .runner as $runner | .work[] | { package, share, runner: $runner }) as $part ({};
            if (.[$part.package].share // -1) < $part.share then .[$part.package] = $part else . end)) as $owners
        | map(
            .runner as $runner
            | .work |= map(.isChecked = ($owners[.package].runner == $runner) | del(.share))
            | .milliseconds = (.milliseconds + ([.work[] | select(.isChecked and .files != []) | $checks[.package]] | add // 0) | round)
        )
    '

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
