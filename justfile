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
    just app/build
    just client/build
    just bridge/build

# regenerate generated code
generate:
    just language/generate
    just app/generate-schema
    just client/generate
    just platform/generate

# format code
format:
    just language/format
    just library/format
    just platform/format
    just app/format
    just client/format
    just bridge/format

# check formatting
format-check:
    just language/format-check
    just library/format-check
    just platform/format-check
    just app/format-check
    just client/format-check
    just bridge/format-check

# alias for format
alias fmt := format

# count source-ish code with generated and fixture corpora excluded
tokei *args:
    command tokei . {{args}} -e target -e node_modules -e .pnpm-store -e .nx -e '*.generated.rs' -e '*.generated.ts' -e '*.generated.js' -e '*.generated.cpp' -e '*.generated.h' -e '*.generated.kt' -e '*.generated.swift' -e 'language/grammar/**/src/parser.c' -e 'language/grammar/**/src/grammar.json' -e 'language/grammar/**/src/node-types.json' -e language/grammar/destack/node_modules -e language/workspace/generated -e app/cli/generated -e bridge/vscode/destack.schema.json

# run static checks
lint:
    just check-hygiene
    just language/lint
    just library/lint
    just platform/lint
    just app/lint
    just client/lint
    just bridge/lint

# run tests
test:
    just language/test
    just library/test
    just platform/test
    just app/test
    just client/test
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
    just app/check-quick
    just client/check-quick
    just bridge/check-quick

# check with slow suites
check-full:
    just check-hygiene
    just language/check-full
    just library/check-full
    just platform/check-full
    just app/check-full
    just client/check-full
    just bridge/check-full

# lint workflows and shell scripts
check-hygiene:
    just ensure-hygiene-toolchain
    PATH="${HOME}/.local/bin:${PATH}" actionlint
    shellcheck -x dev/toolchain/*.sh dev/toolchain/lib/*.sh dev/ci/*.sh app/cli/scripts/*.sh client/scripts/*.sh bridge/zed/scripts/*.sh
    shfmt -d dev/toolchain/*.sh dev/toolchain/lib/*.sh dev/ci/*.sh app/cli/scripts/*.sh client/scripts/*.sh bridge/zed/scripts/*.sh

# install ci hygiene toolchains on this host
install-hygiene-toolchain:
    bash dev/ci/hygiene-toolchain.sh install

# inspect ci hygiene toolchain readiness on this host
doctor-hygiene-toolchain:
    bash dev/ci/hygiene-toolchain.sh doctor

# ensure ci hygiene toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
ensure-hygiene-toolchain:
    bash dev/ci/hygiene-toolchain.sh ensure

# install all toolchains used by runtime, clients, bridges, and ci hygiene lanes
install-toolchain:
    just language/install-toolchain
    just client/install-toolchain
    just bridge/install-toolchain
    just install-hygiene-toolchain

# inspect runtime, clients, bridges, and ci hygiene toolchain readiness on this host
doctor-toolchain:
    just language/doctor-toolchain
    just client/doctor-toolchain
    just bridge/doctor-toolchain
    just doctor-hygiene-toolchain

# ensure runtime, clients, bridges, and ci hygiene toolchains are present
ensure-toolchain:
    just language/ensure-toolchain
    just client/ensure-toolchain
    just bridge/ensure-toolchain
    just ensure-hygiene-toolchain

# clean build artifacts
clean:
    just language/clean
    just library/clean
    just platform/clean
    just app/clean
    just client/clean
    just bridge/clean

# --- release ---

# show current version
version:
    @node -p 'JSON.parse(require("node:fs").readFileSync("destack.json", "utf8")).version'

# show current stability
stability:
    @node -p 'JSON.parse(require("node:fs").readFileSync("destack.json", "utf8")).products.destack.stability'

# set an explicit calendar version
set-version version:
    cargo run --release -p destack_cli -- dev version set "{{version}}"
    just sync-version

# advance the calendar version for the current UTC month
next-version:
    cargo run --release -p destack_cli -- dev version next
    just sync-version

# synchronize generated release metadata
sync-version:
    cargo metadata --format-version 1 > /dev/null
    just client/generate
    bun platform/site/scripts/generate-release.ts

# validate release version and tracked file versions
validate-release tag="":
    bash dev/ci/validate-release.sh "{{tag}}"

# publish packages, dry run by default
publish dry="--dry-run":
    just build
    just library/publish "{{dry}}"
    just app/publish "{{dry}}"
    just client/publish "{{dry}}"
    just bridge/publish "{{dry}}"
    just template/publish-create-destack "{{dry}}"

# publish packages
publish-release:
    just build
    just app/validate-cli-publish
    just _publish-release

# publish live packages under the current stability
_publish-release:
    stability="$(just stability)"; \
    case "${stability}" in \
        experimental|alpha|beta) npm_tag="${stability}"; prerelease="--pre-release" ;; \
        stable) npm_tag="latest"; prerelease="" ;; \
        *) echo "invalid release stability '${stability}'" >&2; exit 1 ;; \
    esac; \
    export NPM_CONFIG_TAG="${npm_tag}"; \
    just library/publish ""; \
    just app/publish ""; \
    just client/publish ""; \
    just bridge/publish "" "${prerelease}"; \
    just template/publish-create-destack-live

# publish packages with local cli binary staging
publish-release-local:
    bash dev/ci/publish-release-local.sh

# create the next weekly release
release:
    bash dev/ci/create-release.sh

# push the current release commit and tag
release-push:
    version="$(just version)"; \
    if ! git rev-parse --verify "v${version}" >/dev/null 2>&1; then \
        echo "error: missing local release tag v${version}" >&2; \
        echo "run: just release" >&2; \
        exit 1; \
    fi; \
    git push origin main; \
    git push origin "v${version}"
