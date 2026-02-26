set shell := ["bash", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

_default:
    @just --list --unsorted

# --- setup ---

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

# run client ci gates
ci-client:
    just client/ci

# run platform ci gates
ci-platform:
    just platform/ci

# run all blocking ci gates locally
ci:
    just check-ci-hygiene
    just ci-language
    just ci-library
    just ci-client
    just ci-platform

# canonical pre commit gate
precommit:
    just ci

# check everything
check:
    just check-ci-hygiene
    just language/check
    just library/check
    just client/check
    just platform/check

# lint ci workflows and shell scripts with strict policy checks
check-ci-hygiene:
    actionlint
    shellcheck .github/scripts/*.sh scripts/toolchain/*.sh scripts/ci/*.sh
    shfmt -d .github/scripts/*.sh scripts/toolchain/*.sh scripts/ci/*.sh
    ./scripts/ci/check-target-policy-sync.sh
    ./scripts/ci/check-branch-protection-check-names.sh
    ./scripts/ci/check-release-tier1-dependencies.sh
    # runtime workflows should go through top-level just wrappers
    if rg -n "run: just language/check-runtime-" .github/workflows/runtime-*.yml; then \
        echo "runtime workflows must call top-level just check-runtime-* wrappers"; \
        exit 1; \
    fi
    # cross compiler env should resolve via toolchain wrappers, not inline zig cc commands
    if rg -n "CC_[A-Za-z0-9_]+.*zig cc -target" justfile language/justfile .github/workflows/*.yml; then \
        echo "inline zig cc toolchain env is not allowed, use scripts/toolchain wrappers"; \
        exit 1; \
    fi
    # tier 1 runtime workflow files should exist
    if [ ! -f .github/workflows/runtime-linux.yml ] || [ ! -f .github/workflows/runtime-windows-gnu.yml ]; then \
        echo "missing required tier 1 runtime workflows"; \
        exit 1; \
    fi
    # ci should call all tier 1 runtime lanes
    if ! rg -n "^  runtime-linux:" .github/workflows/ci.yml >/dev/null; then \
        echo "ci.yml missing runtime-linux tier 1 lane"; \
        exit 1; \
    fi
    if ! rg -n "^  runtime-macos:" .github/workflows/ci.yml >/dev/null; then \
        echo "ci.yml missing runtime-macos tier 1 lane"; \
        exit 1; \
    fi
    if ! rg -n "^  runtime-windows:" .github/workflows/ci.yml >/dev/null; then \
        echo "ci.yml missing runtime-windows tier 1 lane"; \
        exit 1; \
    fi
    if ! rg -n "^  runtime-windows-gnu:" .github/workflows/ci.yml >/dev/null; then \
        echo "ci.yml missing runtime-windows-gnu tier 1 lane"; \
        exit 1; \
    fi
    # runtime linux tier 1 lane should test both host architectures
    if ! rg -n "arch: x86_64" .github/workflows/runtime-linux.yml >/dev/null; then \
        echo "runtime-linux.yml missing x86_64 host lane"; \
        exit 1; \
    fi
    if ! rg -n "arch: aarch64" .github/workflows/runtime-linux.yml >/dev/null; then \
        echo "runtime-linux.yml missing aarch64 host lane"; \
        exit 1; \
    fi
    # tier 1 rows in target policy should include linux and windows gnu
    if ! rg -n "^\\| `x86_64-unknown-linux-gnu` \\| Tier 1 \\|" TARGETS.md >/dev/null; then \
        echo "TARGETS.md must keep x86_64-unknown-linux-gnu in Tier 1"; \
        exit 1; \
    fi
    if ! rg -n "^\\| `aarch64-unknown-linux-gnu` \\| Tier 1 \\|" TARGETS.md >/dev/null; then \
        echo "TARGETS.md must keep aarch64-unknown-linux-gnu in Tier 1"; \
        exit 1; \
    fi
    if ! rg -n "^\\| `x86_64-pc-windows-gnu` \\| Tier 1 \\|" TARGETS.md >/dev/null; then \
        echo "TARGETS.md must keep x86_64-pc-windows-gnu in Tier 1"; \
        exit 1; \
    fi

# apply github branch protection for tier 1 runtime checks
apply-branch-protection *args:
    ./scripts/ci/apply-branch-protection.sh {{args}}

# build everything
build:
    just language/build
    just client/build
    just platform/build

# format all code
format:
    just language/format
    just library/format
    just client/format
    just platform/format

# alias for format
alias fmt := format

# clean all build artifacts
clean:
    just language/clean
    just library/clean
    just client/clean
    just platform/clean

# --- test ---

# run quick tests for local development
test-quick:
    just language/test-quick
    just library/test-quick
    just client/test-quick
    just platform/test-quick

# run blocking ci test gates
test-ci:
    just language/test-ci
    just library/test-ci
    just client/test-ci
    just platform/test-ci

# run nightly test gates
test-nightly:
    just language/test-nightly
    just library/test-nightly
    just client/test-nightly
    just platform/test-nightly

# run release test gates
test-release:
    just language/test-release
    just library/test-release
    just client/test-release
    just platform/test-release

# run quick tests (alias for test-quick)
test:
    just test-quick

# run ide integration tests
test-ide:
    just platform/test-ide

# run language runtime windows target tests through wine
test-runtime-windows-cross:
    just language/test-runtime-windows-cross

# inspect runtime target toolchain readiness on this host
runtime-toolchain-doctor:
    just language/runtime-toolchain-doctor

# bootstrap runtime target toolchains and host prerequisites where possible
runtime-toolchain-bootstrap:
    just language/runtime-toolchain-bootstrap

# run language runtime android target checks
check-runtime-android:
    just language/check-runtime-android

# run language runtime ios target checks
check-runtime-ios:
    just language/check-runtime-ios

# run language runtime wasip1 target checks
check-runtime-wasip1:
    just language/check-runtime-wasip1

# run language runtime linux host checks
check-runtime-linux:
    just language/check-runtime-linux

# run language runtime macos host checks
check-runtime-macos:
    just language/check-runtime-macos

# run language runtime windows host checks
check-runtime-windows-host:
    just language/check-runtime-windows-host

# run language runtime windows gnu target checks with runnable tests
check-runtime-windows-gnu:
    just language/check-runtime-windows-gnu

# run language runtime windows target checks via zig cross
check-runtime-windows-cross:
    just language/check-runtime-windows-cross

# run language resolver windows target tests through wine
test-windows-resolver:
    just language/test-windows-resolver

# run language runtime cross-target checks
check-runtime-cross-targets:
    just language/check-runtime-cross-targets
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
    @cat VERSION

# bump version: major, minor, or patch
bump kind:
    cargo run --release -p destack_cli -- version {{ kind }}

# publish all packages (dry-run by default)
publish dry="--dry-run":
    just build
    just templates/publish-create-destack "{{dry}}"
    just library/publish "{{dry}}"
    just client/publish "{{dry}}"
    just platform/publish "{{dry}}"

# publish all packages as dry run
publish-dry:
    just publish --dry-run

# publish all packages live
publish-live:
    just build
    just platform/validate-cli-publish
    just templates/publish-create-destack ""
    just library/publish ""
    just client/publish ""
    just platform/publish ""

# publish all packages live with local cli binary staging
publish-live-local:
    #!/usr/bin/env bash
    set -euo pipefail

    # prefer staged release artifacts when available
    cli_artifacts_directory="${DESTACK_CLI_ARTIFACTS:-release-cli-assets}"

    just build

    if [ -d "${cli_artifacts_directory}" ]; then
        just platform/stage-cli-binaries-from-artifacts "$(cat VERSION)" "${cli_artifacts_directory}"
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

        just platform/build-cli-binaries
        (cd platform/cli && npm run stage:binaries)
    fi

    just platform/validate-cli-publish
    just templates/publish-create-destack ""
    just library/publish ""
    just client/publish ""
    just platform/publish ""

# create a new release (bump, commit, tag, push)
release kind message:
    #!/usr/bin/env bash
    set -euo pipefail

    # bump version
    just bump {{ kind }}
    VERSION=$(cat VERSION)

    # check if changelog has entry for this version
    if ! grep -q "## \[${VERSION}\]" CHANGELOG.md; then
        echo "Warning: No changelog entry found for version ${VERSION}"
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Aborting release."
            exit 1
        fi
    fi

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
