#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL=C

if [ "$#" -ge 1 ]; then
	TSPP_DRY_FLAG="$1"
else
	TSPP_DRY_FLAG="--dry-run"
fi
TSPP_VERSION_INPUT="${2:-${TSPP_VERSION:-}}"

# resolve repository paths from the script location
TSPP_SCRIPT_DIRECTORY="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TSPP_ZED_DIRECTORY="$(cd "${TSPP_SCRIPT_DIRECTORY}/.." && pwd)"
TSPP_BRIDGE_DIRECTORY="$(cd "${TSPP_ZED_DIRECTORY}/.." && pwd)"
TSPP_REPOSITORY_ROOT="$(cd "${TSPP_BRIDGE_DIRECTORY}/../.." && pwd)"

# configure zed registry defaults
TSPP_ZED_EXTENSION_ID="${TSPP_ZED_EXTENSION_ID:-tspp}"
TSPP_ZED_EXTENSION_SUBMODULE_PATH="${TSPP_ZED_EXTENSION_SUBMODULE_PATH:-extensions/${TSPP_ZED_EXTENSION_ID}}"
TSPP_ZED_EXTENSION_REPOSITORY="${TSPP_ZED_EXTENSION_REPOSITORY:-https://github.com/destack-sh/destack.git}"
TSPP_ZED_EXTENSION_PATH="${TSPP_ZED_EXTENSION_PATH:-language/bridge/zed}"
TSPP_ZED_REGISTRY_UPSTREAM="${TSPP_ZED_REGISTRY_UPSTREAM:-zed-industries/extensions}"
TSPP_ZED_REGISTRY_PUSH_TO="${TSPP_ZED_REGISTRY_PUSH_TO:-}"
TSPP_ZED_REGISTRY_BASE_BRANCH="${TSPP_ZED_REGISTRY_BASE_BRANCH:-main}"
TSPP_ZED_SOURCE_REF="${TSPP_ZED_SOURCE_REF:-HEAD}"
TSPP_ZED_COMMITTER_NAME="${TSPP_ZED_COMMITTER_NAME:-destack-bot}"
TSPP_ZED_COMMITTER_EMAIL="${TSPP_ZED_COMMITTER_EMAIL:-noreply@destack.sh}"

# print an informational message
info() {
	printf '%s\n' "info: $*"
}

# print an error message and exit
fail() {
	printf '%s\n' "error: $*" >&2
	exit 1
}

# ensure a required command exists
require_command() {
	local command_name="$1"
	if ! command -v "${command_name}" >/dev/null 2>&1; then
		fail "required command not found: ${command_name}"
	fi
}

# resolve the release version
resolve_version() {
	if [ -n "${TSPP_VERSION_INPUT}" ]; then
		printf '%s\n' "${TSPP_VERSION_INPUT#v}"
		return
	fi

	local manifest_path="${TSPP_REPOSITORY_ROOT}/package.json"
	if [ -f "${manifest_path}" ]; then
		local version_file
		version_file="$(node -e 'const fs = require("node:fs"); const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); console.log(manifest.version);' "${manifest_path}")"
		if [ -n "${version_file}" ]; then
			printf '%s\n' "${version_file#v}"
			return
		fi
	fi

	fail "release version not provided and package.json is missing"
}

# resolve the version in language/bridge/zed/extension.toml
resolve_manifest_version() {
	local manifest_path="${TSPP_REPOSITORY_ROOT}/language/bridge/zed/extension.toml"
	if [ ! -f "${manifest_path}" ]; then
		fail "zed extension manifest not found: ${manifest_path}"
	fi

	local manifest_version
	manifest_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "${manifest_path}" | head -n 1)"
	if [ -z "${manifest_version}" ]; then
		fail "failed to parse version from ${manifest_path}"
	fi

	printf '%s\n' "${manifest_version}"
}

# validate semver format for release values
validate_semver() {
	local version_value="$1"
	if [[ ! "${version_value}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
		fail "invalid semver value: ${version_value}"
	fi
}

# detect dry run mode from the first argument
is_dry_run() {
	[ "${TSPP_DRY_FLAG}" = "--dry-run" ]
}

# configure gh auth for live mode
configure_live_auth() {
	if ! gh auth status >/dev/null 2>&1; then
		fail "gh auth is required for live mode, export GH_TOKEN or run gh auth login"
	fi

	if ! gh auth setup-git >/dev/null 2>&1; then
		fail "failed to configure git authentication through gh"
	fi
}

# upsert and sort zed registry metadata files
update_registry_metadata() {
	local registry_directory="$1"
	local release_version="$2"

	python3 - "${registry_directory}/extensions.toml" "${registry_directory}/.gitmodules" "${TSPP_ZED_EXTENSION_ID}" "${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" "${TSPP_ZED_EXTENSION_PATH}" "${release_version}" "${TSPP_ZED_EXTENSION_REPOSITORY}" <<'PY'
import re
import sys
from pathlib import Path

extensions_toml_path = Path(sys.argv[1])
gitmodules_path = Path(sys.argv[2])
extension_id = sys.argv[3]
submodule_path = sys.argv[4]
extension_path = sys.argv[5]
release_version = sys.argv[6]
extension_repository = sys.argv[7]

section_pattern = re.compile(r"(?ms)^\[(?P<header>[^\]]+)\]\n(?P<body>(?:^(?!\[).*(?:\n|$))*)")


def parse_sections(text: str):
    sections = []
    for match in section_pattern.finditer(text):
        header = match.group("header")
        body = match.group("body").rstrip("\n")
        sections.append((header, body))
    if not sections:
        raise RuntimeError("failed to parse sectioned config file")
    return sections


def upsert_section(sections, header: str, body: str):
    updated = []
    replaced = False
    for current_header, current_body in sections:
        if current_header == header:
            updated.append((header, body))
            replaced = True
        else:
            updated.append((current_header, current_body))
    if not replaced:
        updated.append((header, body))
    return updated


def sort_sections(sections):
    return sorted(sections, key=lambda pair: pair[0].casefold())


def serialize_sections(sections):
    output_chunks = []
    for header, body in sections:
        output_chunks.append(f"[{header}]\n")
        if body:
            output_chunks.append(f"{body}\n")
        output_chunks.append("\n")
    return "".join(output_chunks).rstrip("\n") + "\n"


extensions_toml_sections = parse_sections(extensions_toml_path.read_text())
extensions_toml_body = "\n".join(
    [
        f'submodule = "{submodule_path}"',
        f'path = "{extension_path}"',
        f'version = "{release_version}"',
    ]
)
extensions_toml_sections = upsert_section(
    sections=extensions_toml_sections,
    header=extension_id,
    body=extensions_toml_body,
)
extensions_toml_sections = sort_sections(extensions_toml_sections)
extensions_toml_path.write_text(serialize_sections(extensions_toml_sections))

gitmodules_sections = parse_sections(gitmodules_path.read_text())
gitmodule_header = f'submodule "{submodule_path}"'
gitmodule_body = "\n".join(
    [
        f"\tpath = {submodule_path}",
        f"\turl = {extension_repository}",
    ]
)
gitmodules_sections = upsert_section(
    sections=gitmodules_sections,
    header=gitmodule_header,
    body=gitmodule_body,
)
gitmodules_sections = sort_sections(gitmodules_sections)
gitmodules_path.write_text(serialize_sections(gitmodules_sections))
PY
}

# render a default pull request body
render_pr_body() {
	local release_version="$1"
	local source_commit="$2"
	cat <<EOF
Update ${TSPP_ZED_EXTENSION_ID} to ${release_version}.

This updates:
- submodule \`${TSPP_ZED_EXTENSION_SUBMODULE_PATH}\`
- \`extensions.toml\` version and path metadata

Source commit:
\`${source_commit}\`
EOF
}

# validate the dry flag format
if [ "${TSPP_DRY_FLAG}" != "--dry-run" ] && [ -n "${TSPP_DRY_FLAG}" ]; then
	fail "invalid dry-run flag: '${TSPP_DRY_FLAG}'"
fi

# verify required local tooling
require_command git
require_command node
require_command python3

# resolve and validate release metadata
TSPP_RELEASE_VERSION="$(resolve_version)"
validate_semver "${TSPP_RELEASE_VERSION}"

TSPP_MANIFEST_VERSION="$(resolve_manifest_version)"
if [ "${TSPP_MANIFEST_VERSION}" != "${TSPP_RELEASE_VERSION}" ]; then
	fail "version mismatch: package.json=${TSPP_RELEASE_VERSION}, language/bridge/zed/extension.toml=${TSPP_MANIFEST_VERSION}"
fi

# resolve the source commit for the zed registry submodule pointer
if ! TSPP_SOURCE_COMMIT="$(git -C "${TSPP_REPOSITORY_ROOT}" rev-parse "${TSPP_ZED_SOURCE_REF}^{commit}" 2>/dev/null)"; then
	fail "failed to resolve source ref: ${TSPP_ZED_SOURCE_REF}"
fi

# clone the zed extension registry into a temporary workspace
TSPP_TEMP_DIRECTORY="$(mktemp -d)"
trap 'rm -rf "${TSPP_TEMP_DIRECTORY}"' EXIT
TSPP_REGISTRY_DIRECTORY="${TSPP_TEMP_DIRECTORY}/zed-extensions"

git clone --filter=blob:none --branch "${TSPP_ZED_REGISTRY_BASE_BRANCH}" "https://github.com/${TSPP_ZED_REGISTRY_UPSTREAM}.git" "${TSPP_REGISTRY_DIRECTORY}" >/dev/null

# create a release branch in the registry clone
TSPP_RELEASE_BRANCH="update-${TSPP_ZED_EXTENSION_ID}-${TSPP_RELEASE_VERSION}"
git -C "${TSPP_REGISTRY_DIRECTORY}" switch --create "${TSPP_RELEASE_BRANCH}" >/dev/null

# add or initialize the extension submodule
if [ ! -e "${TSPP_REGISTRY_DIRECTORY}/${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" ]; then
	git -C "${TSPP_REGISTRY_DIRECTORY}" submodule add "${TSPP_ZED_EXTENSION_REPOSITORY}" "${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" >/dev/null
fi

# update the extension submodule pointer to the release commit
git -C "${TSPP_REGISTRY_DIRECTORY}" submodule sync -- "${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" >/dev/null
git -C "${TSPP_REGISTRY_DIRECTORY}" submodule update --init -- "${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" >/dev/null
git -C "${TSPP_REGISTRY_DIRECTORY}/${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" fetch --tags origin >/dev/null
if ! git -C "${TSPP_REGISTRY_DIRECTORY}/${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" fetch origin "${TSPP_SOURCE_COMMIT}" >/dev/null 2>&1; then
	fail "source commit ${TSPP_SOURCE_COMMIT} is not on ${TSPP_ZED_EXTENSION_REPOSITORY}, push it or set TSPP_ZED_SOURCE_REF"
fi

git -C "${TSPP_REGISTRY_DIRECTORY}/${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" checkout "${TSPP_SOURCE_COMMIT}" >/dev/null

# upsert and sort registry metadata files
update_registry_metadata "${TSPP_REGISTRY_DIRECTORY}" "${TSPP_RELEASE_VERSION}"

# stage all registry changes for commit
git -C "${TSPP_REGISTRY_DIRECTORY}" add "${TSPP_ZED_EXTENSION_SUBMODULE_PATH}" extensions.toml .gitmodules

# stop early when there is no registry delta
if git -C "${TSPP_REGISTRY_DIRECTORY}" diff --cached --quiet; then
	info "zed registry is already up to date for ${TSPP_RELEASE_VERSION}"
	exit 0
fi

# commit the prepared registry delta
git -C "${TSPP_REGISTRY_DIRECTORY}" config user.name "${TSPP_ZED_COMMITTER_NAME}"
git -C "${TSPP_REGISTRY_DIRECTORY}" config user.email "${TSPP_ZED_COMMITTER_EMAIL}"
git -C "${TSPP_REGISTRY_DIRECTORY}" commit -m "Update ${TSPP_ZED_EXTENSION_ID} to ${TSPP_RELEASE_VERSION}" >/dev/null

# print local commit details for dry mode
if is_dry_run; then
	info "dry run: prepared zed registry commit on ${TSPP_RELEASE_BRANCH}"
	git -C "${TSPP_REGISTRY_DIRECTORY}" --no-pager show --stat --oneline HEAD
	exit 0
fi

# validate push target settings for live mode
require_command gh
configure_live_auth

if [ -z "${TSPP_ZED_REGISTRY_PUSH_TO}" ]; then
	fail "missing TSPP_ZED_REGISTRY_PUSH_TO for live mode, expected <owner>/<repo>"
fi

if [[ "${TSPP_ZED_REGISTRY_PUSH_TO}" != */* ]]; then
	fail "invalid TSPP_ZED_REGISTRY_PUSH_TO value: ${TSPP_ZED_REGISTRY_PUSH_TO}"
fi

# configure the push remote for the operator fork
if git -C "${TSPP_REGISTRY_DIRECTORY}" remote get-url push-target >/dev/null 2>&1; then
	git -C "${TSPP_REGISTRY_DIRECTORY}" remote set-url push-target "https://github.com/${TSPP_ZED_REGISTRY_PUSH_TO}.git"
else
	git -C "${TSPP_REGISTRY_DIRECTORY}" remote add push-target "https://github.com/${TSPP_ZED_REGISTRY_PUSH_TO}.git"
fi

# push the release branch to the operator fork
git -C "${TSPP_REGISTRY_DIRECTORY}" push --force-with-lease push-target "${TSPP_RELEASE_BRANCH}:${TSPP_RELEASE_BRANCH}"

# create or reuse the upstream pull request
TSPP_HEAD_OWNER="${TSPP_ZED_REGISTRY_PUSH_TO%%/*}"
TSPP_PR_HEAD="${TSPP_HEAD_OWNER}:${TSPP_RELEASE_BRANCH}"
TSPP_PR_TITLE="Update ${TSPP_ZED_EXTENSION_ID} to ${TSPP_RELEASE_VERSION}"
TSPP_PR_BODY="$(render_pr_body "${TSPP_RELEASE_VERSION}" "${TSPP_SOURCE_COMMIT}")"

TSPP_EXISTING_PR_URL="$(gh pr list --repo "${TSPP_ZED_REGISTRY_UPSTREAM}" --base "${TSPP_ZED_REGISTRY_BASE_BRANCH}" --head "${TSPP_PR_HEAD}" --json url --jq '.[0].url')"

if [ -n "${TSPP_EXISTING_PR_URL}" ] && [ "${TSPP_EXISTING_PR_URL}" != "null" ]; then
	info "zed registry pull request already exists: ${TSPP_EXISTING_PR_URL}"
	exit 0
fi

TSPP_CREATED_PR_URL="$(gh pr create --repo "${TSPP_ZED_REGISTRY_UPSTREAM}" --base "${TSPP_ZED_REGISTRY_BASE_BRANCH}" --head "${TSPP_PR_HEAD}" --title "${TSPP_PR_TITLE}" --body "${TSPP_PR_BODY}")"
info "created zed registry pull request: ${TSPP_CREATED_PR_URL}"
