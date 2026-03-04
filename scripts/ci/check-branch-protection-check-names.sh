#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"

required_checks_file="${script_directory}/required-checks.txt"
ci_file="${repository_root}/.github/workflows/ci.yml"
runtime_linux_file="${repository_root}/.github/workflows/runtime-linux.yml"
runtime_macos_file="${repository_root}/.github/workflows/runtime-macos.yml"
runtime_windows_file="${repository_root}/.github/workflows/runtime-windows.yml"
runtime_windows_gnu_file="${repository_root}/.github/workflows/runtime-windows-gnu.yml"
targets_file="${repository_root}/TARGETS.md"

read_required_checks() {
	sed '/^[[:space:]]*$/d' "${required_checks_file}" | sort -u
}

read_workflow_checks() {
	local macos_check
	local windows_check
	local windows_gnu_check

	# ci hygiene check name comes from the root ci workflow
	if ! rg -n '^    name: CI Hygiene$' "${ci_file}" >/dev/null; then
		echo "ci.yml is missing \"CI Hygiene\" job name" >&2
		exit 1
	fi

	# runtime host check names come from reusable runtime workflows
	macos_check="$(rg -o '^    name: Runtime macOS$' "${runtime_macos_file}" | sed 's/^    name: //')"
	windows_check="$(rg -o '^    name: Runtime Windows$' "${runtime_windows_file}" | sed 's/^    name: //')"
	windows_gnu_check="$(rg -o '^    name: Runtime Windows GNU$' "${runtime_windows_gnu_file}" | sed 's/^    name: //')"

	if [ -z "${macos_check}" ] || [ -z "${windows_check}" ] || [ -z "${windows_gnu_check}" ]; then
		echo "runtime workflow job names are missing required check labels" >&2
		exit 1
	fi

	# runtime linux expands one check per matrix arch
	linux_arches_file="$(mktemp)"
	rg -o 'arch: [a-z0-9_]+' "${runtime_linux_file}" | awk '{print $2}' | sort -u >"${linux_arches_file}"
	if [ ! -s "${linux_arches_file}" ]; then
		rm -f "${linux_arches_file}"
		echo "runtime-linux.yml is missing architecture matrix values" >&2
		exit 1
	fi

	# emit derived workflow check contexts
	echo "CI Hygiene"
	echo "${macos_check}"
	echo "${windows_check}"
	echo "${windows_gnu_check}"
	while IFS= read -r linux_arch; do
		echo "Runtime Linux (${linux_arch})"
	done <"${linux_arches_file}"

	rm -f "${linux_arches_file}"
}

read_targets_tier1_checks() {
	awk -F'|' '
    $0 ~ /^### Tier 1 required checks$/ { in_checks = 1; next }
    in_checks && $0 ~ /^### / { in_checks = 0 }
    in_checks && $0 ~ /^\| `[^`]+` \|$/ {
        check_name = $2
        gsub(/^[ \t]+|[ \t]+$/, "", check_name)
        gsub(/`/, "", check_name)
        print check_name
    }
    ' "${targets_file}" | sort -u
}

compare_check_sets() {
	local label="$1"
	local expected_file="$2"
	local actual_file="$3"
	local missing_file="$4"
	local extra_file="$5"

	comm -23 "${expected_file}" "${actual_file}" >"${missing_file}" || true
	comm -13 "${expected_file}" "${actual_file}" >"${extra_file}" || true

	if [ -s "${missing_file}" ] || [ -s "${extra_file}" ]; then
		echo "${label} is out of sync" >&2

		if [ -s "${missing_file}" ]; then
			echo "missing values:" >&2
			sed 's/^/  - /' "${missing_file}" >&2
		fi

		if [ -s "${extra_file}" ]; then
			echo "unexpected values:" >&2
			sed 's/^/  - /' "${extra_file}" >&2
		fi

		exit 1
	fi
}

if [ ! -f "${required_checks_file}" ]; then
	echo "missing required checks file: ${required_checks_file}" >&2
	exit 1
fi

required_checks_sorted_file="$(mktemp)"
workflow_checks_sorted_file="$(mktemp)"
targets_checks_sorted_file="$(mktemp)"
missing_file="$(mktemp)"
extra_file="$(mktemp)"

trap 'rm -f "${required_checks_sorted_file}" "${workflow_checks_sorted_file}" "${targets_checks_sorted_file}" "${missing_file}" "${extra_file}"' EXIT

read_required_checks >"${required_checks_sorted_file}"
read_workflow_checks | sort -u >"${workflow_checks_sorted_file}"
read_targets_tier1_checks >"${targets_checks_sorted_file}"

compare_check_sets \
	"branch protection required checks vs workflow-derived checks" \
	"${required_checks_sorted_file}" \
	"${workflow_checks_sorted_file}" \
	"${missing_file}" \
	"${extra_file}"

compare_check_sets \
	"branch protection required checks vs TARGETS.md tier 1 required checks" \
	"${required_checks_sorted_file}" \
	"${targets_checks_sorted_file}" \
	"${missing_file}" \
	"${extra_file}"
