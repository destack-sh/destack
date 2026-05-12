#!/bin/bash
#
# diff vendored cranelift against upstream wasmtime
#
# examples:
#   ./diff-upstream.sh                    # diff code changes against v44.0.1
#   ./diff-upstream.sh --stat             # summary of code changes
#   ./diff-upstream.sh --list             # list changed code files
#   ./diff-upstream.sh --all              # include Cargo.toml changes
#   ./diff-upstream.sh --crate codegen    # diff only codegen crate
#   ./diff-upstream.sh --no-format        # diff without formatting either side
#   ./diff-upstream.sh v41.0.0            # diff against different version

set -e

# defaults
UPSTREAM_TAG="v44.0.1"
MODE="diff"
INCLUDE_BUILD_FILES=false
CRATE_FILTER=""
COLOR="auto"
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/cranelift-diff"
FORMAT=true

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS] [UPSTREAM_TAG]

Diff vendored cranelift against upstream wasmtime.

By default, only shows code changes (.rs files). Cargo.toml changes are
expected (workspace dependency rewiring) and hidden unless --all is used.

Options:
  --stat          Show diffstat summary
  --list          List changed files only
  --all           Include Cargo.toml and other build files
  --crate NAME    Filter to specific crate (e.g., codegen, frontend)
  --no-format     Don't format upstream and local Rust sources before diffing
  --no-cache      Don't cache upstream clone
  --clear-cache   Clear cached upstream clones
  --no-color      Disable colored output
  -h, --help      Show this help

Arguments:
  UPSTREAM_TAG    Git tag/branch to diff against (default: $UPSTREAM_TAG)

Examples:
  $(basename "$0")                     Show code changes vs $UPSTREAM_TAG
  $(basename "$0") --stat              Quick summary
  $(basename "$0") --all --list        List all changed files including Cargo.toml
  $(basename "$0") --crate codegen     Only show codegen crate changes
  $(basename "$0") main                Compare against latest main

Exit codes:
  0  No local modifications
  1  Local modifications found

Note: regalloc2 is vendored from crates.io (no git tags available for diffing)
EOF
    exit 0
}

# parse args
USE_CACHE=true
while [[ $# -gt 0 ]]; do
    case $1 in
        --stat)         MODE="stat"; shift ;;
        --list)         MODE="list"; shift ;;
        --all)          INCLUDE_BUILD_FILES=true; shift ;;
        --crate)        CRATE_FILTER="$2"; shift 2 ;;
        --no-format)    FORMAT=false; shift ;;
        --no-cache)     USE_CACHE=false; shift ;;
        --clear-cache)  rm -rf "$CACHE_DIR"; echo "Cache cleared."; exit 0 ;;
        --no-color)     COLOR="never"; shift ;;
        -h|--help)      usage ;;
        -*)             echo "Unknown option: $1" >&2; exit 1 ;;
        *)              UPSTREAM_TAG="$1"; shift ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
RUSTFMT_CONFIG="$REPO_ROOT/rustfmt.toml"

# color support
if [[ "$COLOR" == "auto" ]]; then
    if [[ -t 1 ]]; then COLOR="always"; else COLOR="never"; fi
fi

dim() { [[ "$COLOR" == "always" ]] && printf "\033[2m%s\033[0m" "$1" || printf "%s" "$1"; }
bold() { [[ "$COLOR" == "always" ]] && printf "\033[1m%s\033[0m" "$1" || printf "%s" "$1"; }
green() { [[ "$COLOR" == "always" ]] && printf "\033[32m%s\033[0m" "$1" || printf "%s" "$1"; }
red() { [[ "$COLOR" == "always" ]] && printf "\033[31m%s\033[0m" "$1" || printf "%s" "$1"; }
cyan() { [[ "$COLOR" == "always" ]] && printf "\033[36m%s\033[0m" "$1" || printf "%s" "$1"; }

format_tree() {
    local path="$1"

    [[ ! -d "$path" ]] && return

    local files=()
    while IFS= read -r -d '' file; do
        files+=("$file")

        if [[ "${#files[@]}" -ge 64 ]]; then
            rustfmt --edition 2024 --unstable-features --skip-children --config-path "$RUSTFMT_CONFIG" "${files[@]}"
            files=()
        fi
    done < <(find "$path" -type f -name '*.rs' -not -path '*/target/*' -print0)

    if [[ "${#files[@]}" -gt 0 ]]; then
        rustfmt --edition 2024 --unstable-features --skip-children --config-path "$RUSTFMT_CONFIG" "${files[@]}"
    fi
}

crate_paths() {
    local crate="$1"

    if [[ "$crate" == "isle" ]]; then
        local_path="$SCRIPT_DIR/isle"
        upstream_path="$UPSTREAM_DIR/cranelift/isle/isle"
    else
        local_path="$SCRIPT_DIR/$crate"
        upstream_path="$UPSTREAM_DIR/cranelift/$crate"
    fi
}

# get upstream
UPSTREAM_DIR=""
if [[ "$USE_CACHE" == "true" ]]; then
    UPSTREAM_DIR="$CACHE_DIR/wasmtime-$UPSTREAM_TAG"
    if [[ -d "$UPSTREAM_DIR" ]]; then
        dim "Using cached wasmtime $UPSTREAM_TAG"
        echo ""
    else
        dim "Fetching wasmtime $UPSTREAM_TAG (will be cached)..."
        echo ""
        mkdir -p "$CACHE_DIR"
        git clone --depth 1 --branch "$UPSTREAM_TAG" --single-branch -q \
            https://github.com/bytecodealliance/wasmtime.git "$UPSTREAM_DIR" 2>/dev/null
    fi
else
    UPSTREAM_DIR=$(mktemp -d)
    trap "rm -rf '$UPSTREAM_DIR'" EXIT
    dim "Fetching wasmtime $UPSTREAM_TAG..."
    echo ""
    git clone --depth 1 --branch "$UPSTREAM_TAG" --single-branch -q \
        https://github.com/bytecodealliance/wasmtime.git "$UPSTREAM_DIR" 2>/dev/null
fi

echo ""

# build pathspec based on filter
build_pathspec() {
    local specs=":(exclude)*.orig :(exclude)fuzz/* :(exclude)filetests/*"

    if [[ "$INCLUDE_BUILD_FILES" != "true" ]]; then
        specs="$specs :(exclude)Cargo.toml :(exclude)*/Cargo.toml"
        specs="$specs :(exclude)Cargo.lock :(exclude)*/Cargo.lock"
        specs="$specs :(exclude)*.md :(exclude)*/*.md"
    fi

    echo "$specs"
}

# get stats for a crate pair, outputs: files insertions deletions (or empty)
get_crate_stats() {
    local local_path="$1"
    local upstream_path="$2"

    [[ ! -d "$local_path" ]] && return
    [[ ! -d "$upstream_path" ]] && return

    local pathspec
    pathspec=$(build_pathspec)

    local stat_output
    stat_output=$(git diff --no-index --stat --color=never \
        "$upstream_path" "$local_path" \
        -- $pathspec 2>/dev/null || true)

    if [[ -n "$stat_output" ]]; then
        local summary
        summary=$(echo "$stat_output" | tail -1)
        local files ins del
        files=$(echo "$summary" | grep -oE '[0-9]+ file' | grep -oE '[0-9]+' || echo 0)
        ins=$(echo "$summary" | grep -oE '[0-9]+ insertion' | grep -oE '[0-9]+' || echo 0)
        del=$(echo "$summary" | grep -oE '[0-9]+ deletion' | grep -oE '[0-9]+' || echo 0)
        [[ "$files" -gt 0 ]] && echo "$files $ins $del"
    fi
}

# output diff for a crate
output_crate() {
    local name="$1"
    local local_path="$2"
    local upstream_path="$3"
    local files="$4"
    local ins="$5"
    local del="$6"

    local pathspec
    pathspec=$(build_pathspec)

    case "$MODE" in
        stat)
            printf "  %-20s %3d files  " "$name" "$files"
            green "+$ins"
            printf " "
            red "-$del"
            echo ""
            ;;
        list)
            bold "$name:"
            echo ""
            git diff --no-index --name-only \
                "$upstream_path" "$local_path" \
                -- $pathspec 2>/dev/null \
                | sed "s|$local_path/||" \
                | sed 's/^/    /'
            echo ""
            ;;
        diff)
            git diff --no-index --color="$COLOR" \
                "$upstream_path" "$local_path" \
                -- $pathspec 2>/dev/null \
                | tail -n +3 \
                | sed "s|a$upstream_path|a/$name|g" \
                | sed "s|b$local_path|b/$name|g" \
                || true
            ;;
    esac
}

# cranelift crates (isle handled specially due to nesting)
CRATES="codegen frontend module native object entity bforest assembler-x64 bitset control reader serde srcgen isle"

if [[ "$FORMAT" == "true" ]]; then
    dim "Formatting compared Cranelift Rust sources..."
    echo ""

    for crate in $CRATES; do
        [[ -n "$CRATE_FILTER" && "$crate" != "$CRATE_FILTER" ]] && continue

        crate_paths "$crate"
        format_tree "$upstream_path"
        format_tree "$local_path"
    done

    echo ""
fi

total_files=0
total_insertions=0
total_deletions=0

# header
bold "Cranelift diff"
printf " "
dim "(local vs $UPSTREAM_TAG"
if [[ "$INCLUDE_BUILD_FILES" != "true" ]]; then
    printf ", code only"
fi
printf ")"
echo ""
echo ""

# first pass: collect stats
crate_data=""
for crate in $CRATES; do
    [[ -n "$CRATE_FILTER" && "$crate" != "$CRATE_FILTER" ]] && continue

    crate_paths "$crate"

    stats=$(get_crate_stats "$local_path" "$upstream_path")
    if [[ -n "$stats" ]]; then
        read -r f i d <<< "$stats"
        total_files=$((total_files + f))
        total_insertions=$((total_insertions + i))
        total_deletions=$((total_deletions + d))
        crate_data="$crate_data$crate:$f:$i:$d "
    fi
done

# no changes?
if [[ "$total_files" -eq 0 ]]; then
    green "✓"
    echo " No local modifications"
    echo ""
    dim "Upstream: wasmtime $UPSTREAM_TAG"
    echo ""
    exit 0
fi

# show summary header for diff mode
if [[ "$MODE" == "diff" ]]; then
    printf "  "
    bold "$total_files"
    printf " files changed, "
    green "+$total_insertions"
    printf " "
    red "-$total_deletions"
    echo ""
    echo ""
fi

# second pass: output
for entry in $crate_data; do
    crate="${entry%%:*}"
    rest="${entry#*:}"
    files="${rest%%:*}"
    rest="${rest#*:}"
    ins="${rest%%:*}"
    del="${rest#*:}"

    crate_paths "$crate"

    output_crate "$crate" "$local_path" "$upstream_path" "$files" "$ins" "$del"
done

# footer
echo ""
bold "━━━ Summary ━━━"
echo ""
echo ""
printf "  Upstream: wasmtime "
cyan "$UPSTREAM_TAG"
echo ""
printf "  Changed:  "
bold "$total_files"
printf " files, "
green "+$total_insertions"
printf " "
red "-$total_deletions"
echo ""

if [[ "$INCLUDE_BUILD_FILES" != "true" ]]; then
    echo ""
    dim "  (Cargo.toml changes hidden, use --all to show)"
    echo ""
fi

exit 1  # changes found
