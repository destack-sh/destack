#!/usr/bin/env bash
set -euo pipefail

mode="${1:-here}"

if [ "$#" -gt 1 ]; then
	echo "usage: $0 [here|all]" >&2
	exit 1
fi

if [ "${mode}" != "here" ] && [ "${mode}" != "all" ]; then
	echo "usage: $0 [here|all]" >&2
	exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"
common_git_directory="$(git rev-parse --path-format=absolute --git-common-dir)"
primary_root="$(cd "${common_git_directory}/.." && pwd)"
source_env_path="${primary_root}/.env.local"

if [ ! -e "${source_env_path}" ]; then
	echo "missing source file: ${source_env_path}" >&2
	echo "create ${source_env_path} first" >&2
	exit 1
fi

has_conflict=0

link_worktree_env() {
	local worktree_path="$1"
	local destination_path
	local existing_target

	if [ ! -d "${worktree_path}" ]; then
		echo "skip missing worktree: ${worktree_path}"
		return 0
	fi

	if [ "${worktree_path}" = "${primary_root}" ]; then
		echo "skip primary worktree: ${worktree_path}"
		return 0
	fi

	destination_path="${worktree_path}/.env.local"

	if [ -L "${destination_path}" ]; then
		existing_target="$(readlink "${destination_path}")"

		if [ "${existing_target}" = "${source_env_path}" ]; then
			echo "already linked: ${destination_path}"
			return 0
		fi

		echo "conflict symlink: ${destination_path} -> ${existing_target}" >&2
		has_conflict=1
		return 0
	fi

	if [ -e "${destination_path}" ]; then
		echo "conflict file: ${destination_path}" >&2
		has_conflict=1
		return 0
	fi

	ln -s "${source_env_path}" "${destination_path}"
	echo "linked: ${destination_path} -> ${source_env_path}"
}

if [ "${mode}" = "here" ]; then
	link_worktree_env "${repo_root}"
else
	while IFS= read -r worktree_path; do
		link_worktree_env "${worktree_path}"
	done < <(git worktree list --porcelain | awk '/^worktree / { sub(/^worktree /, ""); print }')
fi

if [ "${has_conflict}" -ne 0 ]; then
	echo "one or more worktrees have an existing .env.local, resolve conflicts and rerun" >&2
	exit 1
fi
