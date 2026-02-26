#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# packaging configuration
DESTACK_VERSION_INPUT="${1:-}"
DESTACK_OUTPUT_DIRECTORY="${2:-}"
DESTACK_TARGETS_INPUT="${DESTACK_RELEASE_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc}"
DESTACK_BINARY_NAMES="destack ds dsc dsx"

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
    if [ -n "${DESTACK_VERSION_INPUT}" ]; then
        printf '%s\n' "${DESTACK_VERSION_INPUT#v}"
        return
    fi

    if [ -f "VERSION" ]; then
        local version_file
        version_file="$(cat VERSION | tr -d '[:space:]')"
        if [ -n "${version_file}" ]; then
            printf '%s\n' "${version_file#v}"
            return
        fi
    fi

    fail "release version not provided and VERSION is missing"
}

# resolve the output directory
resolve_output_directory() {
    if [ -n "${DESTACK_OUTPUT_DIRECTORY}" ]; then
        printf '%s\n' "${DESTACK_OUTPUT_DIRECTORY}"
        return
    fi

    printf '%s\n' "app/cli/install/artifacts"
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

# collect built binaries for one target into a package directory
stage_target_files() {
    local target_triple="$1"
    local package_directory="$2"

    local binary_extension
    binary_extension="$(resolve_binary_extension "${target_triple}")"
    local source_directory="target/${target_triple}/release"

    mkdir -p "${package_directory}"

    local binary_name
    for binary_name in ${DESTACK_BINARY_NAMES}; do
        local source_path="${source_directory}/${binary_name}${binary_extension}"
        local destination_name="${binary_name}${binary_extension}"

        if [ ! -f "${source_path}" ]; then
            fail "missing binary for ${target_triple}: ${source_path}"
        fi

        cp "${source_path}" "${package_directory}/${destination_name}"

        if [ -z "${binary_extension}" ]; then
            chmod 755 "${package_directory}/${destination_name}"
        fi
    done
}

# create one target archive
create_target_archive() {
    local version_value="$1"
    local target_triple="$2"
    local output_directory="$3"
    local temp_directory="$4"

    local package_name="destack-${version_value}-${target_triple}"
    local package_directory="${temp_directory}/${package_name}"
    local archive_extension
    archive_extension="$(resolve_archive_extension "${target_triple}")"
    local archive_name="${package_name}.${archive_extension}"
    local archive_path="${output_directory}/${archive_name}"

    stage_target_files "${target_triple}" "${package_directory}"

    if [ "${archive_extension}" = "zip" ]; then
        (
            cd "${temp_directory}"
            zip -q -r "${archive_path}" "${package_name}"
        )
        return
    fi

    (
        cd "${temp_directory}"
        tar -czf "${archive_path}" "${package_name}"
    )
}

# compute sha256 for a file
sha256_file() {
    local file_path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${file_path}" | awk '{print $1}'
        return
    fi

    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "${file_path}" | awk '{print $1}'
        return
    fi

    fail "sha256 tool not found, install sha256sum or shasum"
}

# write sha256 sums for all archives
write_checksums() {
    local output_directory="$1"
    local checksums_path="${output_directory}/SHA256SUMS"

    : > "${checksums_path}"

    local archive_file
    for archive_file in "${output_directory}"/destack-*; do
        if [ ! -f "${archive_file}" ]; then
            continue
        fi

        local archive_name
        archive_name="$(basename "${archive_file}")"
        local hash_value
        hash_value="$(sha256_file "${archive_file}")"
        printf '%s  %s\n' "${hash_value}" "${archive_name}" >> "${checksums_path}"
    done
}

# clear stale artifacts for the same version
clear_version_artifacts() {
    local version_value="$1"
    local output_directory="$2"

    find "${output_directory}" -maxdepth 1 -type f -name "destack-${version_value}-*" -delete
    find "${output_directory}" -maxdepth 1 -type f -name "SHA256SUMS" -delete
}

# package all configured targets
main() {
    require_command tar
    require_command zip
    require_command awk
    require_command find

    local version_value
    version_value="$(resolve_version)"
    local output_directory
    output_directory="$(resolve_output_directory)"
    local temp_directory
    temp_directory="$(mktemp -d)"
    trap 'rm -rf "${temp_directory:-}"' EXIT

    mkdir -p "${output_directory}"
    clear_version_artifacts "${version_value}" "${output_directory}"

    local target_triple
    for target_triple in ${DESTACK_TARGETS_INPUT}; do
        create_target_archive "${version_value}" "${target_triple}" "${output_directory}" "${temp_directory}"
    done

    write_checksums "${output_directory}"
}

main "$@"
