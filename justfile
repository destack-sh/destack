set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

_default:
    @just --list --unsorted

# --- setup ---

# link this worktree .env.local to the primary repo .env.local
link-env:
    bash dev/ci/link-worktree-env-local.sh here

# link all worktree .env.local files to the primary repo .env.local
link-env-all:
    bash dev/ci/link-worktree-env-local.sh all

# install all dependencies
install:
    bun install
    just language/install

# update all dependencies
update:
    bun update
    cargo update

# --- build ---

# build everything
build:
    just language/build
    just service/build
    just app/build
    just bridge/build

# format all code
format:
    just language/format
    just library/format
    just service/format
    just app/format
    just bridge/format

# check formatting
format-check:
    just language/format-check
    just library/format-check
    just service/format-check
    just app/format-check
    just bridge/format-check

# alias for format
alias fmt := format

# count source-ish code with generated and fixture corpora excluded
tokei *args:
    command tokei . {{args}} -e target -e node_modules -e .pnpm-store -e .nx -e language/test/fixtures/conformance -e '*.generated.rs' -e '*.generated.ts' -e '*.generated.js' -e '*.generated.cpp' -e '*.generated.h' -e '*.generated.kt' -e '*.generated.swift' -e 'language/grammar/**/src/parser.c' -e 'language/grammar/**/src/grammar.json' -e 'language/grammar/**/src/node-types.json' -e language/grammar/destack/node_modules -e language/workspace/generated -e app/cli/generated -e bridge/vscode/destack.schema.json

# run repository static checks
check:
    just check-hygiene
    just check-release-drift
    just language/check
    just library/check
    just service/check
    just app/check
    just bridge/check

# run area test aggregates
test:
    just language/test
    just library/test
    just service/test
    just app/test
    just bridge/test

# run the repository quick gate
quick:
    just check-hygiene
    just check-release-drift
    just language/quick
    just library/quick
    just service/quick
    just app/quick
    just bridge/quick

# run the repository full gate
full:
    just check-hygiene
    just language/full
    just library/full
    just service/full
    just app/full
    just bridge/full

# lint workflows and shell scripts with strict policy checks
check-hygiene:
    just ensure-hygiene-toolchain
    PATH="${HOME}/.local/bin:${PATH}" actionlint
    shellcheck -x .github/scripts/*.sh dev/toolchain/*.sh dev/toolchain/lib/*.sh dev/ci/*.sh app/scripts/*.sh bridge/scripts/*.sh language/scripts/*.sh
    shfmt -d .github/scripts/*.sh dev/toolchain/*.sh dev/toolchain/lib/*.sh dev/ci/*.sh app/scripts/*.sh bridge/scripts/*.sh language/scripts/*.sh
    just check-workflow-policy

# validate ci workflow and target policy architecture
check-workflow-policy:
    bash dev/ci/check-workflow-policy.sh

# validate local release metadata drift between VERSION.txt and CHANGELOG.md
check-release-drift:
    bash dev/ci/validate-release-drift.sh

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

# apply github branch protection for tier 1 runtime checks
apply-branch-protection *args:
    bash dev/ci/apply-branch-protection.sh {{args}}

# clean all build artifacts
clean:
    just language/clean
    just library/clean
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

# generate or refresh the changelog entry for VERSION.txt
generate-release-changelog:
    bash dev/ci/update-changelog.sh "$(cat VERSION.txt)"

# validate release version, tracked file versions, and changelog entry
validate-release tag="":
    bash dev/ci/validate-release.sh "{{tag}}"

# publish all packages (dry-run by default)
publish dry="--dry-run":
    just build
    just library/publish "{{dry}}"
    just app/publish "{{dry}}"
    just bridge/publish "{{dry}}"
    just template/publish-create-destack "{{dry}}"

# publish all packages live
publish-release:
    just build
    just app/validate-cli-publish
    just library/publish ""
    just app/publish ""
    just bridge/publish ""
    just template/publish-create-destack-live

# publish all packages live with local cli binary staging
publish-release-local:
    bash dev/ci/publish-release-local.sh

# create a new release (bump, validate, changelog, commit, tag)
release kind="patch":
    bash dev/ci/create-release.sh "{{kind}}"

# push the current release commit and tag
release-push:
    bash dev/ci/release-push.sh
