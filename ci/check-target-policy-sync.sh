#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/.." && pwd)"

targets_file="${repository_root}/TARGETS.md"
ci_file="${repository_root}/.github/workflows/ci.yml"
nightly_file="${repository_root}/.github/workflows/nightly.yml"
release_file="${repository_root}/.github/workflows/release.yml"

extract_policy_lanes() {
	local allowed_tiers_pattern="$1"

	awk -F'|' '
    $0 ~ /^## Runtime$/ { in_runtime = 1; next }
    in_runtime && $0 ~ /^## / { in_runtime = 0 }
    in_runtime && $0 ~ /^\| `[^`]+` / {
        tier = $3
        lane = $4
        gsub(/^[ \t]+|[ \t]+$/, "", tier)
        gsub(/^[ \t]+|[ \t]+$/, "", lane)
        gsub(/`/, "", lane)
        if (lane != "" && lane != "none yet" && tier ~ tiers) print lane
    }
    ' tiers="${allowed_tiers_pattern}" "${targets_file}" | sort -u
}

extract_workflow_lanes() {
	local workflow_file="$1"

	rg -o '^[[:space:]]{2}runtime-[a-z0-9-]+:' "${workflow_file}" |
		rg -v 'release-smoke' |
		sed -E 's/^[[:space:]]*//; s/:$//' |
		sort -u
}

print_list() {
	local input_file="$1"

	if [ -s "${input_file}" ]; then
		sed 's/^/  - /' "${input_file}"
	fi
}

check_exact_lane_mapping() {
	local label="$1"
	local policy_file="$2"
	local workflow_file="$3"
	local missing_file="$4"
	local extra_file="$5"

	comm -23 "${policy_file}" "${workflow_file}" >"${missing_file}" || true
	comm -13 "${policy_file}" "${workflow_file}" >"${extra_file}" || true

	if [ -s "${missing_file}" ] || [ -s "${extra_file}" ]; then
		echo "${label} runtime lane map is out of sync with TARGETS.md" >&2

		if [ -s "${missing_file}" ]; then
			echo "missing in ${label}:" >&2
			print_list "${missing_file}" >&2
		fi

		if [ -s "${extra_file}" ]; then
			echo "extra in ${label}:" >&2
			print_list "${extra_file}" >&2
		fi

		exit 1
	fi
}

ensure_workflow_files_exist() {
	local lanes_file="$1"
	local workflow_file
	local workflow_name

	while IFS= read -r lane; do
		workflow_name="${lane}"
		workflow_file="${repository_root}/.github/workflows/${workflow_name}.yml"

		if [ -f "${workflow_file}" ]; then
			continue
		fi

		if [[ "${lane}" == *-check ]]; then
			workflow_name="${lane%-check}"
			workflow_file="${repository_root}/.github/workflows/${workflow_name}.yml"
		fi

		if [ ! -f "${workflow_file}" ]; then
			echo "missing workflow file for policy lane: ${lane}" >&2
			exit 1
		fi
	done <"${lanes_file}"
}

policy_lanes_file="$(mktemp)"
ci_policy_lanes_file="$(mktemp)"
release_policy_lanes_file="$(mktemp)"
ci_lanes_file="$(mktemp)"
nightly_lanes_file="$(mktemp)"
release_lanes_file="$(mktemp)"
missing_file="$(mktemp)"
extra_file="$(mktemp)"

trap 'rm -f "${policy_lanes_file}" "${ci_policy_lanes_file}" "${release_policy_lanes_file}" "${ci_lanes_file}" "${nightly_lanes_file}" "${release_lanes_file}" "${missing_file}" "${extra_file}"' EXIT

# policy lanes
extract_policy_lanes '^(Tier 1|Tier 2|Tier 3)$' >"${policy_lanes_file}"
extract_policy_lanes '^(Tier 1|Tier 2)$' >"${ci_policy_lanes_file}"
extract_policy_lanes '^(Tier 1|Tier 2)$' >"${release_policy_lanes_file}"

if [ ! -s "${policy_lanes_file}" ]; then
	echo "TARGETS.md runtime lane table is empty" >&2
	exit 1
fi

# workflow lanes
extract_workflow_lanes "${ci_file}" >"${ci_lanes_file}"
extract_workflow_lanes "${nightly_file}" >"${nightly_lanes_file}"
extract_workflow_lanes "${release_file}" >"${release_lanes_file}"

# make sure every policy lane has a reusable runtime workflow
ensure_workflow_files_exist "${policy_lanes_file}"

# keep ci and release aligned with Tier 1 and Tier 2 policy lanes
check_exact_lane_mapping "ci.yml" "${ci_policy_lanes_file}" "${ci_lanes_file}" "${missing_file}" "${extra_file}"
check_exact_lane_mapping "release.yml" "${release_policy_lanes_file}" "${release_lanes_file}" "${missing_file}" "${extra_file}"

# keep nightly aligned with the full policy lane set, including Tier 3
check_exact_lane_mapping "nightly.yml" "${policy_lanes_file}" "${nightly_lanes_file}" "${missing_file}" "${extra_file}"
