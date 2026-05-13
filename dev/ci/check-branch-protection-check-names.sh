#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"

required_checks_file="${script_directory}/required-checks.txt"
ci_file="${repository_root}/.github/workflows/ci.yml"
runtime_linux_file="${repository_root}/.github/workflows/runtime-linux-check.yml"

read_required_checks() {
	sed '/^[[:space:]]*$/d' "${required_checks_file}" | sort -u
}

read_workflow_checks() {
	# ci hygiene check name comes from the root ci workflow
	if ! rg -n '^    name: Hygiene Check$' "${ci_file}" >/dev/null; then
		echo "ci.yml is missing \"Hygiene Check\" job name" >&2
		exit 1
	fi

	# runtime linux expands one check per matrix arch
	linux_arches_file="$(mktemp)"
	rg -o 'arch: [a-z0-9_]+' "${runtime_linux_file}" | awk '{print $2}' | sort -u >"${linux_arches_file}"
	if [ ! -s "${linux_arches_file}" ]; then
		rm -f "${linux_arches_file}"
		echo "runtime-linux-check.yml is missing architecture matrix values" >&2
		exit 1
	fi

	# emit derived workflow check contexts
	echo "Hygiene Check"
	echo "Language Resolver Check (Windows)"
	while IFS= read -r linux_arch; do
		echo "Runtime Check (Linux, ${linux_arch})"
	done <"${linux_arches_file}"

	rm -f "${linux_arches_file}"
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
missing_file="$(mktemp)"
extra_file="$(mktemp)"

trap 'rm -f "${required_checks_sorted_file}" "${workflow_checks_sorted_file}" "${missing_file}" "${extra_file}"' EXIT

read_required_checks >"${required_checks_sorted_file}"
read_workflow_checks | sort -u >"${workflow_checks_sorted_file}"

compare_check_sets \
	"branch protection required checks vs workflow-derived checks" \
	"${required_checks_sorted_file}" \
	"${workflow_checks_sorted_file}" \
	"${missing_file}" \
	"${extra_file}"
