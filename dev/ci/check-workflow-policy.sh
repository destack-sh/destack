#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
cd "${repository_root}"

ci_file="${repository_root}/.github/workflows/ci.yml"
nightly_file="${repository_root}/.github/workflows/nightly.yml"
release_file="${repository_root}/.github/workflows/release.yml"
runtime_workflow_files=("${repository_root}/.github/workflows/runtime-"*.yml)
workflow_files=("${repository_root}/.github/workflows/"*.yml)

# shellcheck disable=SC1091
source "${repository_root}/dev/toolchain/versions.sh"

recipe_list="$(just --list --unsorted)"

assert_required_recipe() {
	local recipe_name="$1"

	if ! printf '%s\n' "${recipe_list}" | rg -n "^\\s*${recipe_name}(\\s|$)" >/dev/null; then
		echo "missing required top level recipe: ${recipe_name}" >&2
		exit 1
	fi
}

assert_required_recipe "install-toolchain"
assert_required_recipe "doctor-toolchain"
assert_required_recipe "ensure-toolchain"
assert_required_recipe "check-hygiene"
assert_required_recipe "check-workflow-policy"
assert_required_recipe "quick"
assert_required_recipe "full"

# runtime workflows should call scoped language recipes, never root wrappers
if rg -n "run: just runtime-" "${runtime_workflow_files[@]}"; then
	echo "runtime workflows must call scoped language runtime recipes" >&2
	exit 1
fi

if rg -n "run: \\./dev/toolchain/runtime-" "${runtime_workflow_files[@]}"; then
	echo "runtime workflows must not call toolchain directly" >&2
	exit 1
fi

if rg -n "run: \\./\\.github/scripts/install-runtime-android-ndk.sh" "${runtime_workflow_files[@]}"; then
	echo "runtime workflows must call just language/install-runtime-android-ndk instead of .github/scripts directly" >&2
	exit 1
fi

# bridge jobs should install with the canonical ensure entrypoint
if rg -n "just bridge/install-toolchain" "${ci_file}" "${nightly_file}" "${release_file}"; then
	echo "workflow bridge toolchain setup must use just bridge/ensure-toolchain" >&2
	exit 1
fi

# release workflow should be tag driven for immutable releases
if ! rg -n '^\s+- "v\*"$' "${release_file}" >/dev/null; then
	echo "release workflow must trigger from v* tags" >&2
	exit 1
fi

if rg -n '^  workflow_dispatch:' "${release_file}" >/dev/null; then
	echo "release workflow must not use workflow_dispatch for publish lanes" >&2
	exit 1
fi

# release workflow should enforce tag/version and tracked version consistency
if ! rg -n 'just validate-release ' "${release_file}" >/dev/null; then
	if ! rg -n "validate-release-tag-version.sh" "${release_file}" >/dev/null; then
		echo "release workflow must validate tag and VERSION.txt consistency" >&2
		exit 1
	fi

	if ! rg -n "dev version check" "${release_file}" >/dev/null; then
		echo "release workflow must run tracked version file checks" >&2
		exit 1
	fi
fi

# workflows should route through shared setup actions
if rg -n "rustup toolchain install|oven-sh/setup-bun@|mlugg/setup-zig@" "${workflow_files[@]}"; then
	echo "workflows must use shared setup actions under .github/actions" >&2
	exit 1
fi

# workflow files should not hardcode pinned toolchain versions
if rg -n -F "${DESTACK_RUST_TOOLCHAIN}" "${workflow_files[@]}"; then
	echo "workflows must not hardcode rust toolchain versions" >&2
	exit 1
fi
if rg -n -F "${DESTACK_BUN_VERSION}" "${workflow_files[@]}"; then
	echo "workflows must not hardcode bun versions" >&2
	exit 1
fi
if rg -n -F "${DESTACK_ZIG_VERSION}" "${workflow_files[@]}"; then
	echo "workflows must not hardcode zig versions" >&2
	exit 1
fi

# cross compiler env should resolve via toolchain wrappers, not inline zig cc commands
if rg -n "CC_[A-Za-z0-9_]+.*zig cc -target" "${repository_root}/justfile" "${repository_root}/language/justfile" "${repository_root}/.github/workflows/"*.yml; then
	echo "inline zig cc toolchain env is not allowed, use toolchain wrappers" >&2
	exit 1
fi

# scheduled and release runtime workflow files should exist
if [ ! -f "${repository_root}/.github/workflows/runtime-linux-check.yml" ] ||
	[ ! -f "${repository_root}/.github/workflows/runtime-macos-check.yml" ] ||
	[ ! -f "${repository_root}/.github/workflows/runtime-windows-check.yml" ] ||
	[ ! -f "${repository_root}/.github/workflows/runtime-ios-check.yml" ] ||
	[ ! -f "${repository_root}/.github/workflows/runtime-android-check.yml" ]; then
	echo "missing required runtime workflows" >&2
	exit 1
fi

# mainline ci should keep the cheap runtime and windows resolver baseline
if ! rg -n "^  runtime-linux-check:" "${ci_file}" >/dev/null; then
	echo "ci.yml missing runtime-linux-check mainline lane" >&2
	exit 1
fi
if ! rg -n "^  language-resolver-windows-check:" "${ci_file}" >/dev/null; then
	echo "ci.yml missing language-resolver-windows-check mainline lane" >&2
	exit 1
fi

# runtime linux tier 1 lane should test both host architectures
if ! rg -n "arch: x86_64" "${repository_root}/.github/workflows/runtime-linux-check.yml" >/dev/null; then
	echo "runtime-linux-check.yml missing x86_64 host lane" >&2
	exit 1
fi
if ! rg -n "arch: aarch64" "${repository_root}/.github/workflows/runtime-linux-check.yml" >/dev/null; then
	echo "runtime-linux-check.yml missing aarch64 host lane" >&2
	exit 1
fi

# nightly and release should keep the full platform coverage
for workflow_file in "${nightly_file}" "${release_file}"; do
	if ! rg -n "^  runtime-android-check:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing runtime-android-check full lane" >&2
		exit 1
	fi
	if ! rg -n "^  runtime-ios-check:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing runtime-ios-check full lane" >&2
		exit 1
	fi
	if ! rg -n "^  runtime-linux-check:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing runtime-linux-check full lane" >&2
		exit 1
	fi
	if ! rg -n "^  runtime-macos-check:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing runtime-macos-check full lane" >&2
		exit 1
	fi
	if ! rg -n "^  runtime-windows-check:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing runtime-windows-check full lane" >&2
		exit 1
	fi
	if ! rg -n "^  language-resolver-windows-check:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing language-resolver-windows-check full lane" >&2
		exit 1
	fi
	if ! rg -n "^  bridge-swift-build:" "${workflow_file}" >/dev/null; then
		echo "$(basename "${workflow_file}") missing bridge-swift-build full lane" >&2
		exit 1
	fi
done

# nightly should publish the rolling canary prerelease from main
if ! rg -n "^  nightly-release-create:" "${nightly_file}" >/dev/null; then
	echo "nightly.yml missing nightly-release-create canary lane" >&2
	exit 1
fi

if ! rg -n "if: github.ref == 'refs/heads/main'" "${nightly_file}" >/dev/null; then
	echo "nightly.yml must guard prerelease publication to main" >&2
	exit 1
fi

# tier 1 rows in target policy should keep the supported host targets
if ! rg -n '^\| `x86_64-unknown-linux-gnu` \| Tier 1 \|' "${repository_root}/TARGETS.md" >/dev/null; then
	echo "TARGETS.md must keep x86_64-unknown-linux-gnu in Tier 1" >&2
	exit 1
fi
if ! rg -n '^\| `aarch64-unknown-linux-gnu` \| Tier 1 \|' "${repository_root}/TARGETS.md" >/dev/null; then
	echo "TARGETS.md must keep aarch64-unknown-linux-gnu in Tier 1" >&2
	exit 1
fi
"${script_directory}/check-target-policy-sync.sh"
"${script_directory}/check-branch-protection-check-names.sh"
"${script_directory}/check-release-tier1-dependencies.sh"
