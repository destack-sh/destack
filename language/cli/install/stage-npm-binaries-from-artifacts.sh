#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# staging configuration
TSPP_SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
TSPP_CLI_DIRECTORY="$(cd -- "${TSPP_SCRIPT_DIRECTORY}/.." >/dev/null 2>&1 && pwd)"
TSPP_VERSION_INPUT="${1:-}"
TSPP_ARTIFACTS_DIRECTORY="${2:-${TSPP_CLI_DIRECTORY}/install/artifacts}"
TSPP_TARGETS_INPUT="${TSPP_RELEASE_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc}"
TSPP_BINARY_NAMES="tspp tsppc"

# print an error message and exit
fail() {
    printf '%s\n' "error: $*" >&2
    exit 1
}

# require a command in path
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

    if [ -f "package.json" ]; then
        local version_file
        version_file="$(node -p 'JSON.parse(require("node:fs").readFileSync("package.json", "utf8")).version')"
        if [ -n "${version_file}" ]; then
            printf '%s\n' "${version_file#v}"
            return
        fi
    fi

    fail "release version not provided and package.json is missing"
}

# resolve a target archive file extension
resolve_archive_extension() {
    local target_triple="$1"
    if [[ "${target_triple}" == *windows* ]]; then
        printf '%s\n' "zip"
        return
    fi

    printf '%s\n' "tar.gz"
}

# resolve binary extension for a target
resolve_binary_extension() {
    local target_triple="$1"
    if [[ "${target_triple}" == *windows* ]]; then
        printf '%s\n' ".exe"
        return
    fi

    printf '%s\n' ""
}

# extract one release archive into a temporary directory
extract_archive() {
    local archive_path="$1"
    local target_triple="$2"
    local extract_directory="$3"
    local archive_extension
    archive_extension="$(resolve_archive_extension "${target_triple}")"

    if [ "${archive_extension}" = "zip" ]; then
        require_command unzip
        unzip -q -o "${archive_path}" -d "${extract_directory}"
        return
    fi

    require_command tar
    tar -xzf "${archive_path}" -C "${extract_directory}"
}

# stage one target archive into target/<triple>/release
stage_target_archive() {
    local version_value="$1"
    local target_triple="$2"
    local temp_directory="$3"

    local archive_extension
    archive_extension="$(resolve_archive_extension "${target_triple}")"
    local archive_name="tspp-${version_value}-${target_triple}.${archive_extension}"
    local archive_path="${TSPP_ARTIFACTS_DIRECTORY}/${archive_name}"

    if [ ! -f "${archive_path}" ]; then
        fail "missing archive for ${target_triple}: ${archive_path}"
    fi

    local extract_directory="${temp_directory}/${target_triple}"
    mkdir -p "${extract_directory}"
    extract_archive "${archive_path}" "${target_triple}" "${extract_directory}"

    local package_directory="${extract_directory}/tspp-${version_value}-${target_triple}"
    if [ ! -d "${package_directory}" ]; then
        fail "archive did not contain expected directory: ${package_directory}"
    fi

    local destination_directory="target/${target_triple}/release"
    mkdir -p "${destination_directory}"

    local binary_extension
    binary_extension="$(resolve_binary_extension "${target_triple}")"

    local binary_name
    for binary_name in ${TSPP_BINARY_NAMES}; do
        local source_path="${package_directory}/${binary_name}${binary_extension}"
        local destination_path="${destination_directory}/${binary_name}${binary_extension}"

        if [ ! -f "${source_path}" ]; then
            fail "missing binary in archive ${archive_name}: ${source_path}"
        fi

        cp "${source_path}" "${destination_path}"

        if [ -z "${binary_extension}" ]; then
            chmod 755 "${destination_path}"
        fi
    done
}

# run archive staging for all targets
main() {
    require_command node

    local version_value
    version_value="$(resolve_version)"

    local temp_directory
    temp_directory="$(mktemp -d)"
    trap 'rm -rf "${temp_directory:-}"' EXIT

    local target_triple
    for target_triple in ${TSPP_TARGETS_INPUT}; do
        stage_target_archive "${version_value}" "${target_triple}" "${temp_directory}"
    done
}

main "$@"
