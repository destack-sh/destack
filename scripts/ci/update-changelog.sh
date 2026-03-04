#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
cd "${repository_root}"

version="${1:-$(tr -d '[:space:]' <VERSION.txt)}"
changelog_path="${repository_root}/CHANGELOG.md"
release_date="$(date -u +%Y-%m-%d)"
current_tag="v${version}"
max_commits_per_group="${DESTACK_CHANGELOG_MAX_COMMITS_PER_GROUP:-25}"

# require strict semver format
if ! [[ "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "invalid version '${version}': expected X.Y.Z" >&2
	exit 1
fi

if ! [[ "${max_commits_per_group}" =~ ^[1-9][0-9]*$ ]]; then
	echo "invalid DESTACK_CHANGELOG_MAX_COMMITS_PER_GROUP '${max_commits_per_group}'" >&2
	exit 1
fi

previous_tag="$(
	git tag --list "v*" --sort=-version:refname |
		rg -v "^${current_tag}$" |
		head -n 1 || true
)"

# collect commit subjects for the release range
release_range="HEAD"
if [ -n "${previous_tag}" ]; then
	release_range="${previous_tag}..HEAD"
fi

features_file="$(mktemp)"
fixes_file="$(mktemp)"
refactors_file="$(mktemp)"
docs_file="$(mktemp)"
tests_file="$(mktemp)"
chores_file="$(mktemp)"
other_file="$(mktemp)"
section_file="$(mktemp)"
temp_file="$(mktemp)"

trap 'rm -f "${features_file}" "${fixes_file}" "${refactors_file}" "${docs_file}" "${tests_file}" "${chores_file}" "${other_file}" "${section_file}" "${temp_file}"' EXIT

while IFS= read -r subject; do
	[ -z "${subject}" ] && continue

	case "${subject}" in
	feat\(* | feat:*)
		printf -- "- %s\n" "${subject}" >>"${features_file}"
		;;
	fix\(* | fix:*)
		printf -- "- %s\n" "${subject}" >>"${fixes_file}"
		;;
	refactor\(* | refactor:*)
		printf -- "- %s\n" "${subject}" >>"${refactors_file}"
		;;
	docs\(* | docs:*)
		printf -- "- %s\n" "${subject}" >>"${docs_file}"
		;;
	test\(* | test:*)
		printf -- "- %s\n" "${subject}" >>"${tests_file}"
		;;
	chore\(* | chore:* | dev\(* | dev:*)
		printf -- "- %s\n" "${subject}" >>"${chores_file}"
		;;
	*)
		printf -- "- %s\n" "${subject}" >>"${other_file}"
		;;
	esac
done < <(git log --no-merges --pretty=format:%s "${release_range}")

append_section_group() {
	local section_title="$1"
	local section_items_file="$2"
	local total_count
	local hidden_count

	if [ ! -s "${section_items_file}" ]; then
		return
	fi

	total_count="$(wc -l <"${section_items_file}" | tr -d '[:space:]')"
	hidden_count=0
	if [ "${total_count}" -gt "${max_commits_per_group}" ]; then
		hidden_count="$((total_count - max_commits_per_group))"
	fi

	{
		printf -- "### %s\n" "${section_title}"
		head -n "${max_commits_per_group}" "${section_items_file}"
		if [ "${hidden_count}" -gt 0 ]; then
			printf -- "- ... and %s more\n" "${hidden_count}"
		fi
		printf -- "\n"
	} >>"${section_file}"
}

{
	printf -- "## [%s] - %s\n\n" "${version}" "${release_date}"

	if [ -n "${previous_tag}" ]; then
		printf -- "_Changes since %s._\n\n" "${previous_tag}"
	else
		printf -- "_Initial release entry._\n\n"
	fi
} >"${section_file}"

append_section_group "Features" "${features_file}"
append_section_group "Fixes" "${fixes_file}"
append_section_group "Refactors" "${refactors_file}"
append_section_group "Docs" "${docs_file}"
append_section_group "Tests" "${tests_file}"
append_section_group "Chores" "${chores_file}"
append_section_group "Other" "${other_file}"

if [ ! -s "${features_file}" ] &&
	[ ! -s "${fixes_file}" ] &&
	[ ! -s "${refactors_file}" ] &&
	[ ! -s "${docs_file}" ] &&
	[ ! -s "${tests_file}" ] &&
	[ ! -s "${chores_file}" ] &&
	[ ! -s "${other_file}" ]; then
	{
		printf -- "### Other\n"
		printf -- "- no commits found in release range\n\n"
	} >>"${section_file}"
fi

# ensure changelog exists with a stable header
if [ ! -f "${changelog_path}" ]; then
	cat >"${changelog_path}" <<'EOF'
# Changelog

This file tracks notable release changes for Destack.
Entries are generated from conventional commit history during release preparation.

EOF
fi

# replace existing section or insert a new one above the first version section
if rg -n "^## \\[${version}\\]" "${changelog_path}" >/dev/null; then
	awk -v version="${version}" -v section_file="${section_file}" '
	BEGIN {
		while ((getline line < section_file) > 0) {
			section = section line ORS
		}
		close(section_file)
	}
	$0 ~ "^## \\[" version "\\]" {
		if (!replaced) {
			printf "%s", section
			replaced = 1
		}
		skip = 1
		next
	}
	skip && /^## \[/ {
		skip = 0
		print
		next
	}
	!skip {
		print
	}
	END {
		if (!replaced) {
			printf "%s", section
		}
	}
	' "${changelog_path}" >"${temp_file}"
else
	awk -v section_file="${section_file}" '
	BEGIN {
		while ((getline line < section_file) > 0) {
			section = section line ORS
		}
		close(section_file)
	}
	/^## \[/ && !inserted {
		printf "%s", section
		inserted = 1
	}
	{
		print
	}
	END {
		if (!inserted) {
			if (NR > 0) {
				print ""
			}
			printf "%s", section
		}
	}
	' "${changelog_path}" >"${temp_file}"
fi

mv "${temp_file}" "${changelog_path}"
echo "updated changelog entry for ${version}"
