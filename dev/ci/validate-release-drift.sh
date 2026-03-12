#!/usr/bin/env bash
set -euo pipefail

changed_files() {
	git diff --name-only --cached -- VERSION.txt CHANGELOG.md
	git diff --name-only -- VERSION.txt CHANGELOG.md
}

base_changed_files() {
	local base_ref=""

	if [ -n "${GITHUB_BASE_REF:-}" ] && git show-ref --verify --quiet "refs/remotes/origin/${GITHUB_BASE_REF}"; then
		base_ref="$(git merge-base HEAD "origin/${GITHUB_BASE_REF}")"
	elif git show-ref --verify --quiet refs/remotes/origin/main; then
		base_ref="$(git merge-base HEAD origin/main)"
	elif git rev-parse --verify HEAD^ >/dev/null 2>&1; then
		base_ref="HEAD^"
	fi

	if [ -z "${base_ref}" ]; then
		return 0
	fi

	git diff --name-only "${base_ref}...HEAD" -- VERSION.txt CHANGELOG.md
}

all_changed_files="$(
	{
		changed_files
		base_changed_files
	} | sort -u
)"

is_version_changed="0"
is_changelog_changed="0"

if printf '%s\n' "${all_changed_files}" | rg -x "VERSION.txt" >/dev/null; then
	is_version_changed="1"
fi

if printf '%s\n' "${all_changed_files}" | rg -x "CHANGELOG.md" >/dev/null; then
	is_changelog_changed="1"
fi

if [ "${is_version_changed}" = "0" ] && [ "${is_changelog_changed}" = "0" ]; then
	exit 0
fi

if [ "${is_version_changed}" = "1" ] && [ "${is_changelog_changed}" = "1" ]; then
	exit 0
fi

echo "VERSION.txt and CHANGELOG.md must change together" >&2
echo "changed files:" >&2
printf '%s\n' "${all_changed_files}" >&2
exit 1
