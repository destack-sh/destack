#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/.." && pwd)"

targets_file="${repository_root}/TARGETS.md"
release_file="${repository_root}/.github/workflows/release.yml"

extract_tier1_runtime_lanes() {
	awk -F'|' '
    $0 ~ /^## Runtime$/ { in_runtime = 1; next }
    in_runtime && $0 ~ /^## / { in_runtime = 0 }
    in_runtime && $0 ~ /^\| `[^`]+` \| Tier 1 \| `[^`]+` \|/ {
        lane = $4
        gsub(/^[ \t]+|[ \t]+$/, "", lane)
        gsub(/`/, "", lane)
        if (lane != "") print lane
    }
    ' "${targets_file}" | sort -u
}

extract_job_needs() {
	local job_name="$1"

	awk -v job="  ${job_name}:" '
    $0 == job { in_job = 1; in_needs = 0; next }
    in_job && $0 ~ /^  [A-Za-z0-9_-]+:$/ { in_job = 0; in_needs = 0 }
    in_job && $0 ~ /^    needs:$/ { in_needs = 1; next }
    in_job && in_needs && $0 ~ /^      - / {
        need = $0
        sub(/^      - /, "", need)
        print need
        next
    }
    in_job && in_needs && $0 !~ /^      - / { in_needs = 0 }
    ' "${release_file}" | sort -u
}

assert_job_has_tier1_lanes() {
	local job_name="$1"
	local tier1_file="$2"
	local needs_file="$3"
	local missing_file="$4"

	extract_job_needs "${job_name}" >"${needs_file}"
	comm -23 "${tier1_file}" "${needs_file}" >"${missing_file}" || true

	if [ -s "${missing_file}" ]; then
		echo "release job ${job_name} is missing tier 1 runtime dependencies:" >&2
		sed 's/^/  - /' "${missing_file}" >&2
		exit 1
	fi
}

tier1_file="$(mktemp)"
needs_file="$(mktemp)"
missing_file="$(mktemp)"

trap 'rm -f "${tier1_file}" "${needs_file}" "${missing_file}"' EXIT

extract_tier1_runtime_lanes >"${tier1_file}"

if [ ! -s "${tier1_file}" ]; then
	echo "TARGETS.md does not define any tier 1 runtime lanes" >&2
	exit 1
fi

assert_job_has_tier1_lanes "release-build" "${tier1_file}" "${needs_file}" "${missing_file}"
assert_job_has_tier1_lanes "cli-build" "${tier1_file}" "${needs_file}" "${missing_file}"
