#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# validation configuration
DESTACK_VERSION_INPUT="${1:-}"
DESTACK_ARTIFACTS_DIRECTORY="${2:-app/cli/install/artifacts}"
DESTACK_TARGETS_INPUT="${DESTACK_RELEASE_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc}"
DESTACK_CHECKSUMS_NAME="SHA256SUMS"

# print an error message and exit
fail() {
    printf '%s\n' "error: $*" >&2
    exit 1
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

# resolve a target archive extension
resolve_archive_extension() {
    local target_triple="$1"
    if [[ "${target_triple}" == *windows* ]]; then
        printf '%s\n' "zip"
        return
    fi

    printf '%s\n' "tar.gz"
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

# resolve the expected archive file path
resolve_archive_path() {
    local version_value="$1"
    local target_triple="$2"
    local archive_extension
    archive_extension="$(resolve_archive_extension "${target_triple}")"

    printf '%s\n' "${DESTACK_ARTIFACTS_DIRECTORY}/destack-${version_value}-${target_triple}.${archive_extension}"
}

# verify one target archive exists and matches checksum
validate_target_archive() {
    local version_value="$1"
    local target_triple="$2"
    local checksums_path="$3"

    local archive_path
    archive_path="$(resolve_archive_path "${version_value}" "${target_triple}")"
    if [ ! -f "${archive_path}" ]; then
        fail "missing archive for ${target_triple}: ${archive_path}"
    fi

    local archive_name
    archive_name="$(basename "${archive_path}")"
    local checksum_line
    checksum_line="$(grep -E "(\\*| )${archive_name}\$" "${checksums_path}" | head -n 1 || true)"
    if [ -z "${checksum_line}" ]; then
        fail "missing checksum entry for ${archive_name}"
    fi

    local expected_hash
    expected_hash="$(printf '%s' "${checksum_line}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"
    local actual_hash
    actual_hash="$(sha256_file "${archive_path}" | tr '[:upper:]' '[:lower:]')"

    if [ "${expected_hash}" != "${actual_hash}" ]; then
        fail "checksum mismatch for ${archive_name}"
    fi
}

# validate all release artifacts
main() {
    local version_value
    version_value="$(resolve_version)"
    local checksums_path="${DESTACK_ARTIFACTS_DIRECTORY}/${DESTACK_CHECKSUMS_NAME}"
    if [ ! -f "${checksums_path}" ]; then
        fail "missing checksums file: ${checksums_path}"
    fi

    local target_triple
    for target_triple in ${DESTACK_TARGETS_INPUT}; do
        validate_target_archive "${version_value}" "${target_triple}" "${checksums_path}"
    done
}

main "$@"
