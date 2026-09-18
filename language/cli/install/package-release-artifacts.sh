#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# packaging configuration
DESTACK_SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
DESTACK_CLI_DIRECTORY="$(cd -- "${DESTACK_SCRIPT_DIRECTORY}/.." >/dev/null 2>&1 && pwd)"
DESTACK_VERSION_INPUT="${1:-}"
DESTACK_OUTPUT_DIRECTORY="${2:-}"
DESTACK_TARGETS_INPUT="${DESTACK_RELEASE_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc}"
DESTACK_RELEASE_TAG_INPUT="${DESTACK_RELEASE_TAG:-}"
DESTACK_RELEASE_CHANNEL_INPUT="${DESTACK_RELEASE_CHANNEL:-release}"
DESTACK_RELEASE_STABILITY_INPUT="${DESTACK_RELEASE_STABILITY:-}"
DESTACK_MANIFEST_NAME="manifest.json"
read -r -a DESTACK_TARGETS <<< "${DESTACK_TARGETS_INPUT}"
DESTACK_BINARY_NAMES=(destack ds dsc)

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

# resolve the release tag
resolve_release_tag() {
    local version_value="$1"

    if [ -n "${DESTACK_RELEASE_TAG_INPUT}" ]; then
        printf '%s\n' "${DESTACK_RELEASE_TAG_INPUT}"
        return
    fi

    printf 'v%s\n' "${version_value}"
}

# resolve the release channel
resolve_release_channel() {
    case "${DESTACK_RELEASE_CHANNEL_INPUT}" in
        release | canary)
            printf '%s\n' "${DESTACK_RELEASE_CHANNEL_INPUT}"
            ;;
        *)
            fail "unsupported release channel: ${DESTACK_RELEASE_CHANNEL_INPUT}"
            ;;
    esac
}

# resolve the release stability
resolve_release_stability() {
    local stability_value="${DESTACK_RELEASE_STABILITY_INPUT}"
    if [ -z "${stability_value}" ] && [ -f "dev/release/config.json" ]; then
        stability_value="$(node -p 'JSON.parse(require("node:fs").readFileSync("dev/release/config.json", "utf8")).stability')"
    fi

    case "${stability_value}" in
        experimental | alpha | beta | stable)
            printf '%s\n' "${stability_value}"
            ;;
        *)
            fail "unsupported release stability: ${stability_value}"
            ;;
    esac
}

# resolve the output directory
resolve_output_directory() {
    if [ -n "${DESTACK_OUTPUT_DIRECTORY}" ]; then
        printf '%s\n' "${DESTACK_OUTPUT_DIRECTORY}"
        return
    fi

    printf '%s\n' "${DESTACK_CLI_DIRECTORY}/install/artifacts"
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
    for binary_name in "${DESTACK_BINARY_NAMES[@]}"; do
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

# resolve the archive file name for a target
resolve_archive_name() {
    local version_value="$1"
    local target_triple="$2"
    local archive_extension
    archive_extension="$(resolve_archive_extension "${target_triple}")"

    printf '%s\n' "destack-${version_value}-${target_triple}.${archive_extension}"
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

# write the release update manifest
write_manifest() {
    local version_value="$1"
    local release_tag="$2"
    local release_channel="$3"
    local release_stability="$4"
    local output_directory="$5"
    local checksums_path="${output_directory}/SHA256SUMS"
    local manifest_path="${output_directory}/${DESTACK_MANIFEST_NAME}"
    local targets_count="${#DESTACK_TARGETS[@]}"
    local target_index=0

    {
        printf '{\n'
        printf '  "version": "%s",\n' "${version_value}"
        printf '  "releaseTag": "%s",\n' "${release_tag}"
        printf '  "channel": "%s",\n' "${release_channel}"
        printf '  "stability": "%s",\n' "${release_stability}"
        printf '  "checksumsFile": "SHA256SUMS",\n'
        printf '  "assets": [\n'

        local target_triple
        for target_triple in "${DESTACK_TARGETS[@]}"; do
            target_index="$((target_index + 1))"

            local archive_name
            archive_name="$(resolve_archive_name "${version_value}" "${target_triple}")"
            local archive_format
            archive_format="$(resolve_archive_extension "${target_triple}")"
            local checksum_line
            checksum_line="$(grep -E "(\\*| )${archive_name}\$" "${checksums_path}" | head -n 1 || true)"
            if [ -z "${checksum_line}" ]; then
                fail "checksum entry not found for ${archive_name}"
            fi
            local archive_sha256
            archive_sha256="$(printf '%s' "${checksum_line}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"

            printf '    {\n'
            printf '      "targetTriple": "%s",\n' "${target_triple}"
            printf '      "archiveName": "%s",\n' "${archive_name}"
            printf '      "archiveFormat": "%s",\n' "${archive_format}"
            printf '      "archiveSha256": "%s",\n' "${archive_sha256}"
            printf '      "binaries": ["destack", "ds", "dsc"]\n'

            if [ "${target_index}" -lt "${targets_count}" ]; then
                printf '    },\n'
            else
                printf '    }\n'
            fi
        done

        printf '  ]\n'
        printf '}\n'
    } > "${manifest_path}"
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
    require_command node

    local version_value
    version_value="$(resolve_version)"
    local release_tag
    release_tag="$(resolve_release_tag "${version_value}")"
    local release_stability
    release_stability="$(resolve_release_stability)"
    local release_channel
    release_channel="$(resolve_release_channel)"
    local output_directory
    output_directory="$(resolve_output_directory)"
    local temp_directory
    temp_directory="$(mktemp -d)"
    trap 'rm -rf "${temp_directory:-}"' EXIT

    mkdir -p "${output_directory}"
    clear_version_artifacts "${version_value}" "${output_directory}"

    local target_triple
    for target_triple in "${DESTACK_TARGETS[@]}"; do
        create_target_archive "${version_value}" "${target_triple}" "${output_directory}" "${temp_directory}"
    done

    write_checksums "${output_directory}"
    write_manifest "${version_value}" "${release_tag}" "${release_channel}" "${release_stability}" "${output_directory}"
}

main "$@"
