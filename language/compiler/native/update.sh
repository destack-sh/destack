#!/bin/bash

set -euo pipefail

UPSTREAM_TAG="${1:-v47.0.3}"
UPSTREAM_VERSION="${UPSTREAM_TAG#v}"
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/cranelift-diff"
SCRIPT_DIRECTORY="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPSTREAM_DIRECTORY="$CACHE_ROOT/wasmtime-$UPSTREAM_TAG"

# fetch the exact Wasmtime release once
if [[ ! -d "$UPSTREAM_DIRECTORY/.git" ]]; then
    mkdir -p "$CACHE_ROOT"
    git clone --depth 1 --branch "$UPSTREAM_TAG" --single-branch \
        https://github.com/bytecodealliance/wasmtime.git "$UPSTREAM_DIRECTORY"
fi

# replace one vendored crate with its exact upstream tree
sync_crate() {
    local source="$1"
    local target="$2"

    rsync --archive --delete "$source/" "$target/"
}

# inherit Destack's toolchain while keeping vendored code outside its lint policy
adapt_manifest() {
    local manifest="$1"

    perl -0pi -e \
        's/\[lints\]\nworkspace = true\n/[lints.clippy]\nall = { level = "allow", priority = -1 }\n/' \
        "$manifest"
}

# update the Cranelift crates linked by the compiler
for crate in \
    assembler-x64 \
    bforest \
    bitset \
    codegen \
    control \
    entity \
    frontend \
    module \
    native \
    object \
    reader \
    serde \
    srcgen
do
    sync_crate "$UPSTREAM_DIRECTORY/cranelift/$crate" "$SCRIPT_DIRECTORY/cranelift/$crate"
done

# ISLE lives one level below the upstream family directory
sync_crate "$UPSTREAM_DIRECTORY/cranelift/isle/isle" "$SCRIPT_DIRECTORY/cranelift/isle"

# update the Wasmtime crates coupled to this Cranelift release
sync_crate "$UPSTREAM_DIRECTORY/pulley" "$SCRIPT_DIRECTORY/pulley"
sync_crate "$UPSTREAM_DIRECTORY/crates/core" "$SCRIPT_DIRECTORY/wasmtime-core"

# adapt upstream workspace manifests to the Destack workspace
while IFS= read -r -d '' manifest; do
    adapt_manifest "$manifest"
done < <(find "$SCRIPT_DIRECTORY/cranelift" "$SCRIPT_DIRECTORY/pulley" \
    "$SCRIPT_DIRECTORY/wasmtime-core" -name Cargo.toml -print0)

perl -0pi -e \
    's/cranelift-isle = \{ path = "\.\.\/isle\/isle", version = "=[^"]+" \}/cranelift-isle = { workspace = true }/' \
    "$SCRIPT_DIRECTORY/cranelift/codegen/Cargo.toml"
perl -0pi -e "s/^version\\.workspace = true$/version = \"$UPSTREAM_VERSION\"/m" \
    "$SCRIPT_DIRECTORY/pulley/Cargo.toml" \
    "$SCRIPT_DIRECTORY/pulley/macros/Cargo.toml" \
    "$SCRIPT_DIRECTORY/wasmtime-core/Cargo.toml"
perl -0pi -e 's/authors\.workspace = true/authors = ["The Wasmtime Project Developers"]/' \
    "$SCRIPT_DIRECTORY/wasmtime-core/Cargo.toml"
cat >> "$SCRIPT_DIRECTORY/pulley/Cargo.toml" <<'EOF'

[[test]]
name = "all"
path = "tests/all/main.rs"
required-features = ["encode"]

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = [
    'cfg(pulley_tail_calls)',
    'cfg(pulley_assume_llvm_makes_tail_calls)',
    'cfg(pulley_disable_interp_simd)',
] }
EOF

printf 'Updated native sources from Wasmtime %s.\n' "$UPSTREAM_TAG"
