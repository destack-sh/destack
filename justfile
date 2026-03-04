set shell := ["bash", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

_default:
    @just --list --unsorted

# --- setup ---

# link this worktree .env.local to the primary repo .env.local
env-link:
    bash scripts/ci/link-worktree-env-local.sh here

# link all worktree .env.local files to the primary repo .env.local
env-link-all:
    bash scripts/ci/link-worktree-env-local.sh all

# install all dependencies
install:
    bun install
    just language/install

# update all dependencies
update:
    bun update
    cargo update

# builtin libs

# typescript lib version is pinned in language/builtin/scripts/versions.sh
fetch-builtin-libs-version:
    just language/fetch-builtin-libs-version

# fetch builtin libs
fetch-builtin-libs:
    just language/fetch-builtin-libs

# generate builtin lib registry
generate-builtin-libs:
    just language/generate-builtin-libs

# --- build ---

# install js workspace dependencies for ci style gates
ci-install:
    bun install --frozen-lockfile

# run language ci gates
ci-language:
    just language/ci

# run library ci gates
ci-library:
    just library/ci

# run service ci gates
ci-service:
    just service/ci

# run app ci gates
ci-app:
    just app/ci

# run bridge ci gates
ci-bridge:
    just bridge/ci

# run all blocking ci gates locally
ci:
    just check-ci-hygiene
    just ci-language
    just ci-library
    just ci-service
    just ci-app
    just ci-bridge

# canonical pre commit gate
precommit:
    just ci

# check everything
check:
    just check-ci-hygiene
    just language/check
    just library/check
    just service/check
    just app/check
    just bridge/check

# lint ci workflows and shell scripts with strict policy checks
check-ci-hygiene:
    just ci-hygiene-toolchain-ensure
    PATH="${HOME}/.local/bin:${PATH}" actionlint
    shellcheck -x .github/scripts/*.sh scripts/toolchain/*.sh scripts/toolchain/lib/*.sh scripts/ci/*.sh bridge/scripts/*.sh
    shfmt -d .github/scripts/*.sh scripts/toolchain/*.sh scripts/toolchain/lib/*.sh scripts/ci/*.sh bridge/scripts/*.sh
    just check-workflow-policy

# validate ci workflow and target policy architecture
check-workflow-policy:
    ./scripts/ci/check-workflow-policy.sh

# install ci hygiene toolchains on this host
ci-hygiene-toolchain-install:
    bash scripts/ci/hygiene-toolchain.sh install

# inspect ci hygiene toolchain readiness on this host
ci-hygiene-toolchain-doctor:
    bash scripts/ci/hygiene-toolchain.sh doctor

# ensure ci hygiene toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
ci-hygiene-toolchain-ensure:
    bash scripts/ci/hygiene-toolchain.sh ensure

# backwards compatibility alias
alias install-ci-hygiene-toolchains := ci-hygiene-toolchain-install

# install all toolchains used by runtime, bridge, and ci hygiene lanes
toolchain-install:
    just runtime-toolchain-install
    just bridge/toolchain-install
    just ci-hygiene-toolchain-install

# inspect runtime, bridge, and ci hygiene toolchain readiness on this host
toolchain-doctor:
    just runtime-toolchain-doctor
    just bridge/toolchain-doctor
    just ci-hygiene-toolchain-doctor

# ensure runtime, bridge, and ci hygiene toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
toolchain-ensure:
    just runtime-toolchain-ensure
    just bridge/toolchain-ensure
    just ci-hygiene-toolchain-ensure

# apply github branch protection for tier 1 runtime checks
apply-branch-protection *args:
    ./scripts/ci/apply-branch-protection.sh {{args}}

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

# alias for format
alias fmt := format

# clean all build artifacts
clean:
    just language/clean
    just library/clean
    just service/clean
    just app/clean
    just bridge/clean

# --- test ---

# run quick tests for local development
test-quick:
    just language/test-quick
    just library/test-quick
    just service/test-quick
    just app/test-quick
    just bridge/test-quick

# run blocking ci test gates
test-ci:
    just language/test-ci
    just library/test-ci
    just service/test-ci
    just app/test-ci
    just bridge/test-ci

# run nightly test gates
test-nightly:
    just language/test-nightly
    just library/test-nightly
    just service/test-nightly
    just app/test-nightly
    just bridge/test-nightly

# run release test gates
test-release:
    just language/test-release
    just library/test-release
    just service/test-release
    just app/test-release
    just bridge/test-release

# run quick tests (alias for test-quick)
test:
    just test-quick

# run ide integration tests
test-ide:
    just bridge/test-ide

# run language runtime windows target tests through wine
runtime-windows-gnu-test *args:
    just language/runtime-windows-gnu-test {{args}}

# run language runtime windows gnu smoke binary through wine
runtime-windows-gnu-smoke *args:
    just language/runtime-windows-gnu-smoke {{args}}

# install runtime target toolchains and host prerequisites where possible
runtime-toolchain-install:
    just language/toolchain-install

# inspect runtime target toolchain readiness on this host
runtime-toolchain-doctor:
    just language/toolchain-doctor

# ensure runtime target toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
runtime-toolchain-ensure:
    just language/toolchain-ensure

# lint runtime toolchain shell scripts
runtime-toolchain-lint:
    just language/toolchain-lint

# backwards compatibility alias
alias runtime-toolchain-bootstrap := runtime-toolchain-install

# run language runtime android target checks
runtime-android-check:
    just language/runtime-android-check

# run language runtime ios target checks
runtime-ios-check:
    just language/runtime-ios-check

# run language runtime wasip1 target checks
runtime-wasip1-check:
    just language/runtime-wasip1-check

# run language runtime linux host checks
runtime-linux-check:
    just language/runtime-linux-check

# run language runtime macos host checks
runtime-macos-check:
    just language/runtime-macos-check

# run language runtime windows host checks
runtime-windows-msvc-check:
    just language/runtime-windows-msvc-check

# run language runtime windows gnu target checks with runnable tests
runtime-windows-gnu-check *args:
    just language/runtime-windows-gnu-check {{args}}

# run language runtime windows target checks via zig cross
runtime-windows-gnu-cross-check:
    just language/runtime-windows-gnu-cross-check

# run language resolver windows target tests through wine
test-windows-resolver:
    just language/test-windows-resolver

# run language runtime cross-target checks
runtime-cross-targets-check:
    just language/runtime-cross-targets-check

# run privileged language runtime platform tests
test-runtime-privileged:
    just language/test-runtime-privileged
# --- bench ---

# run benchmarks
bench:
    just language/bench

# run nightly benchmark lane
bench-nightly:
    just language/bench-nightly

# --- fuzz ---

# run fuzzers
fuzz duration="60":
    just language/fuzz {{duration}}

# run nightly fuzz smoke lane
fuzz-nightly duration="120":
    just language/fuzz-nightly {{duration}}

# --- release ---

# show current version
version:
    @cat VERSION.txt

# bump version: major, minor, or patch
bump kind:
    cargo run --release -p destack_cli -- dev version {{ kind }}

# generate or refresh the changelog entry for VERSION.txt
release-changelog:
    bash scripts/ci/update-changelog.sh "$(cat VERSION.txt)"

# validate release version, tracked file versions, and changelog entry
release-validate tag="":
    #!/usr/bin/env bash
    set -euo pipefail

    version="$(cat VERSION.txt)"
    release_tag="{{tag}}"
    if [ -z "${release_tag}" ]; then
        release_tag="v${version}"
    fi

    bash scripts/ci/validate-release-tag-version.sh "${release_tag}"
    cargo run --release -p destack_cli -- dev version check
    bash scripts/ci/validate-release-changelog.sh "${version}"

# publish all packages (dry-run by default)
publish dry="--dry-run":
    just build
    just library/publish "{{dry}}"
    just app/publish "{{dry}}"
    just bridge/publish "{{dry}}"
    just template/publish-create-destack "{{dry}}"

# publish all packages as dry run
publish-dry:
    just publish --dry-run

# publish all packages live
publish-live:
    just build
    just app/validate-cli-publish
    just publish-live-packages

# publish all packages live without build orchestration
publish-live-packages:
    just library/publish-live
    just app/publish-live
    just bridge/publish-live
    just template/publish-create-destack-live

# publish all packages live with local cli binary staging
publish-live-local:
    #!/usr/bin/env bash
    set -euo pipefail

    # prefer staged release artifacts when available
    cli_artifacts_directory="${DESTACK_CLI_ARTIFACTS:-release-cli-assets}"

    just build

    if [ -d "${cli_artifacts_directory}" ]; then
        just app/stage-cli-binaries-from-artifacts "$(cat VERSION.txt)" "${cli_artifacts_directory}"
    else
        # default local target selection to host target when not explicitly set
        if [ -z "${DESTACK_RELEASE_TARGETS:-}" ]; then
            host_target="$(rustc -vV | awk '/^host: / { print $2 }')"
            case "${host_target}" in
                aarch64-apple-darwin|x86_64-apple-darwin|aarch64-unknown-linux-gnu|x86_64-unknown-linux-gnu|x86_64-pc-windows-msvc)
                    export DESTACK_RELEASE_TARGETS="${host_target}"
                    ;;
                *)
                    echo "error: unsupported host target for default local publish: ${host_target}" >&2
                    echo "set DESTACK_RELEASE_TARGETS explicitly to one or more supported targets" >&2
                    exit 1
                    ;;
            esac
        fi

        just app/build-cli-binaries
        (cd app/cli && npm run stage:binaries)
    fi

    just app/validate-cli-publish
    just publish-live-packages

# create a new release (bump, commit, tag, push)
release kind message:
    #!/usr/bin/env bash
    set -euo pipefail

    # bump version
    just bump {{ kind }}
    just release-changelog
    VERSION=$(cat VERSION.txt)
    just release-validate "v${VERSION}"

    # stage and commit
    git add -A
    git commit -m "{{ message }}"

    # create tag
    git tag -a "v${VERSION}" -m "Release v${VERSION}"

    echo ""
    echo "Release v${VERSION} created locally."
    echo "To publish:"
    echo "  git push origin main --tags"
    echo "  just publish-live"
    echo "  just publish-live-local   # uses release-cli-assets when present, else host target"
