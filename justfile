set shell := ["bash", "-cu"]
set dotenv-load := true
set dotenv-filename := ".env.local"

_default:
    @just --list --unsorted

# --- setup ---

# link this worktree .env.local to the primary repo .env.local
link-env:
    bash scripts/ci/link-worktree-env-local.sh here

# link all worktree .env.local files to the primary repo .env.local
link-env-all:
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

# check everything
check:
    just check-hygiene
    just language/check
    just library/check
    just service/check
    just app/check
    just bridge/check

# run all scoped tests
test:
    just language/test
    just library/test
    just service/test
    just app/test
    just bridge/test

# run the fast repository gate
quick:
    just check-hygiene
    just language/quick
    just library/quick
    just service/quick
    just app/quick
    just bridge/quick

# run the full repository gate
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
    shellcheck -x .github/scripts/*.sh scripts/toolchain/*.sh scripts/toolchain/lib/*.sh scripts/ci/*.sh bridge/scripts/*.sh
    shfmt -d .github/scripts/*.sh scripts/toolchain/*.sh scripts/toolchain/lib/*.sh scripts/ci/*.sh bridge/scripts/*.sh
    just check-workflow-policy

# validate ci workflow and target policy architecture
check-workflow-policy:
    ./scripts/ci/check-workflow-policy.sh

# install ci hygiene toolchains on this host
install-hygiene-toolchain:
    bash scripts/ci/hygiene-toolchain.sh install

# inspect ci hygiene toolchain readiness on this host
doctor-hygiene-toolchain:
    bash scripts/ci/hygiene-toolchain.sh doctor

# ensure ci hygiene toolchains are present, optionally auto install with DESTACK_AUTO_INSTALL_TOOLCHAINS=1
ensure-hygiene-toolchain:
    bash scripts/ci/hygiene-toolchain.sh ensure

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
    ./scripts/ci/apply-branch-protection.sh {{args}}

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
bump kind:
    cargo run --release -p destack_cli -- dev version {{ kind }}

# generate or refresh the changelog entry for VERSION.txt
generate-release-changelog:
    bash scripts/ci/update-changelog.sh "$(cat VERSION.txt)"

# validate release version, tracked file versions, and changelog entry
validate-release tag="":
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
    just library/publish ""
    just app/publish ""
    just bridge/publish ""
    just template/publish-create-destack-live

# create a new release (bump, validate, changelog, commit, tag)
release kind:
    #!/usr/bin/env bash
    set -euo pipefail

    # bump version
    just bump {{ kind }}
    just generate-release-changelog
    VERSION=$(cat VERSION.txt)
    just validate-release "v${VERSION}"

    # stage and commit
    release_commit_message="chore(all): bump version to ${VERSION}"
    git add -A
    git commit -m "${release_commit_message}"

    # create tag
    git tag -a "v${VERSION}" -m "Release v${VERSION}"

    echo ""
    echo "Release v${VERSION} created locally."
    echo "To publish:"
    echo "  just push-release"
    echo "  just publish-release"
    echo "  just publish-release-local   # uses release-cli-assets when present, else host target"

# push the current release commit and tag
push-release:
    #!/usr/bin/env bash
    set -euo pipefail

    version="$(cat VERSION.txt)"
    if ! git rev-parse --verify "v${version}" >/dev/null 2>&1; then
        echo "error: missing local release tag v${version}" >&2
        echo "run: just release <major|minor|patch>" >&2
        exit 1
    fi

    git push origin main
    git push origin "v${version}"
