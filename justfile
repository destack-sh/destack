set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

_default:
    @just --list --unsorted

# --- setup ---

# install dependencies
install:
    bun install

# update dependencies
update:
    bun update
    cargo update

# --- build ---

# build everything
build:
    just language/build
    just platform/build
    just service/build
    just app/build
    just bridge/build

# format code
format:
    just language/format
    just library/format
    just platform/format
    just service/format
    just app/format
    just bridge/format

# check formatting
format-check:
    just language/format-check
    just library/format-check
    just platform/format-check
    just service/format-check
    just app/format-check
    just bridge/format-check

# alias for format
alias fmt := format

# count source-ish code with generated and fixture corpora excluded
tokei *args:
    command tokei . {{args}} -e target -e node_modules -e .pnpm-store -e .nx -e language/test/fixtures/conformance -e '*.generated.rs' -e '*.generated.ts' -e '*.generated.js' -e '*.generated.cpp' -e '*.generated.h' -e '*.generated.kt' -e '*.generated.swift' -e 'language/grammar/**/src/parser.c' -e 'language/grammar/**/src/grammar.json' -e 'language/grammar/**/src/node-types.json' -e language/grammar/destack/node_modules -e language/workspace/generated -e app/cli/generated -e bridge/vscode/destack.schema.json

# run static checks
lint:
    just check-hygiene
    just language/lint
    just library/lint
    just platform/lint
    just service/lint
    just app/lint
    just bridge/lint

# run tests
test:
    just language/test
    just library/test
    just platform/test
    just service/test
    just app/test
    just bridge/test

# default check
check:
    just check-quick

# normal check
check-quick:
    just check-hygiene
    just language/check-quick
    just library/check-quick
    just platform/check-quick
    just service/check-quick
    just app/check-quick
    just bridge/check-quick

# check with slow suites
check-full:
    just check-hygiene
    just language/check-full
    just library/check-full
    just platform/check-full
    just service/check-full
    just app/check-full
    just bridge/check-full

# lint workflows and shell scripts
check-hygiene:
    just ensure-hygiene-toolchain
    PATH="${HOME}/.local/bin:${PATH}" actionlint
    shellcheck -x dev/toolchain/*.sh dev/toolchain/lib/*.sh dev/ci/*.sh app/scripts/*.sh bridge/scripts/*.sh
    shfmt -d dev/toolchain/*.sh dev/toolchain/lib/*.sh dev/ci/*.sh app/scripts/*.sh bridge/scripts/*.sh

# install ci hygiene toolchains on this host
install-hygiene-toolchain:
    bash dev/ci/hygiene-toolchain.sh install

# inspect ci hygiene toolchain readiness on this host
doctor-hygiene-toolchain:
    bash dev/ci/hygiene-toolchain.sh doctor

# ensure ci hygiene toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
ensure-hygiene-toolchain:
    bash dev/ci/hygiene-toolchain.sh ensure

# install all toolchains used by runtime, bridge, and ci hygiene lanes
install-toolchain:
    just language/install-toolchain
    just bridge/install-toolchain
    just install-hygiene-toolchain

# inspect runtime, bridge, and ci hygiene toolchain readiness on this host
doctor-toolchain:
    just language/doctor-toolchain
    just bridge/doctor-toolchain
    just doctor-hygiene-toolchain

# ensure runtime, bridge, and ci hygiene toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
ensure-toolchain:
    just language/ensure-toolchain
    just bridge/ensure-toolchain
    just ensure-hygiene-toolchain

# clean build artifacts
clean:
    just language/clean
    just library/clean
    just platform/clean
    just service/clean
    just app/clean
    just bridge/clean

# --- release ---

# show current version
version:
    @cat VERSION.txt

# bump version: major, minor, or patch
bump kind="patch":
    cargo run --release -p destack_cli -- dev version {{ kind }}

# validate release version and tracked file versions
validate-release tag="":
    bash dev/ci/validate-release.sh "{{tag}}"

# publish packages, dry run by default
publish dry="--dry-run":
    just build
    just library/publish "{{dry}}"
    just app/publish "{{dry}}"
    just bridge/publish "{{dry}}"
    just template/publish-create-destack "{{dry}}"

# publish packages
publish-release:
    just build
    just app/validate-cli-publish
    just library/publish ""
    just app/publish ""
    just bridge/publish ""
    just template/publish-create-destack-live

# publish packages with local cli binary staging
publish-release-local:
    bash dev/ci/publish-release-local.sh

# create a new release (bump, validate, commit, tag)
release kind="patch":
    bash dev/ci/create-release.sh "{{kind}}"

# push the current release commit and tag
release-push:
    version="$(cat VERSION.txt)"; \
    if ! git rev-parse --verify "v${version}" >/dev/null 2>&1; then \
        echo "error: missing local release tag v${version}" >&2; \
        echo "run: just release <major|minor|patch>" >&2; \
        exit 1; \
    fi; \
    git push origin main; \
    git push origin "v${version}"
