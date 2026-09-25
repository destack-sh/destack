#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# release source
TSPP_REPOSITORY="${TSPP_REPOSITORY:-destack-sh/tspp}"
TSPP_RELEASE_BASE_URL="${TSPP_RELEASE_BASE_URL:-https://github.com/${TSPP_REPOSITORY}/releases/download}"
TSPP_API_BASE_URL="${TSPP_API_BASE_URL:-https://api.github.com/repos/${TSPP_REPOSITORY}}"
TSPP_CURL_USER_AGENT="${TSPP_CURL_USER_AGENT:-tspp-installer}"
TSPP_GITHUB_TOKEN="${TSPP_GITHUB_TOKEN:-}"

# install configuration
TSPP_VERSION_INPUT="${TSPP_VERSION:-latest}"
TSPP_INSTALL_DIR="${TSPP_INSTALL:-${HOME}/.tspp/bin}"
TSPP_NO_MODIFY_PATH="${TSPP_NO_MODIFY_PATH:-0}"

# resolved runtime metadata
TSPP_OS=""
TSPP_ARCH=""
TSPP_LIBC=""
TSPP_TARGET_TRIPLE=""
TSPP_VERSION_VALUE=""
TSPP_RELEASE_TAG=""
TSPP_ARCHIVE_NAME=""
TSPP_ARCHIVE_URL=""
TSPP_CHECKSUMS_NAME="SHA256SUMS"
TSPP_CHECKSUMS_URL=""

# print an informational message
info() {
    printf '%s\n' "info: $*"
}

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

# download a file with retries
download_file() {
    local url="$1"
    local destination="$2"

    require_command curl

    # include github auth when a token is configured
    if [ -n "${TSPP_GITHUB_TOKEN}" ]; then
        curl --fail --location --retry 3 --retry-delay 1 --retry-all-errors --silent --show-error \
            --user-agent "${TSPP_CURL_USER_AGENT}" \
            --header "authorization: Bearer ${TSPP_GITHUB_TOKEN}" \
            "${url}" \
            --output "${destination}"
        return
    fi

    # download without auth headers by default
    curl --fail --location --retry 3 --retry-delay 1 --retry-all-errors --silent --show-error \
        --user-agent "${TSPP_CURL_USER_AGENT}" \
        "${url}" \
        --output "${destination}"
}

# fetch a json string with retries
fetch_json() {
    local url="$1"

    require_command curl

    # include github auth when a token is configured
    if [ -n "${TSPP_GITHUB_TOKEN}" ]; then
        curl --fail --location --retry 3 --retry-delay 1 --retry-all-errors --silent --show-error \
            --user-agent "${TSPP_CURL_USER_AGENT}" \
            --header "authorization: Bearer ${TSPP_GITHUB_TOKEN}" \
            "${url}"
        return
    fi

    # fetch without auth headers by default
    curl --fail --location --retry 3 --retry-delay 1 --retry-all-errors --silent --show-error \
        --user-agent "${TSPP_CURL_USER_AGENT}" \
        "${url}"
}

# parse the release tag from github release json
parse_release_tag() {
    local release_json="$1"

    # parse with jq when available
    if command -v jq >/dev/null 2>&1; then
        printf '%s' "${release_json}" | jq -r '.tag_name // empty'
        return
    fi

    # parse with sed when jq is unavailable
    printf '%s' "${release_json}" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1
}

# resolve the current os identifier
resolve_os() {
    local uname_output
    uname_output="$(uname -s)"
    case "${uname_output}" in
        Darwin)
            TSPP_OS="darwin"
            ;;
        Linux)
            TSPP_OS="linux"
            ;;
        *)
            fail "unsupported operating system: ${uname_output}"
            ;;
    esac
}

# resolve the current architecture identifier
resolve_arch() {
    local uname_output
    uname_output="$(uname -m)"
    case "${uname_output}" in
        arm64|aarch64)
            TSPP_ARCH="arm64"
            ;;
        x86_64|amd64)
            TSPP_ARCH="x64"
            ;;
        *)
            fail "unsupported architecture: ${uname_output}"
            ;;
    esac
}

# resolve the linux libc family
resolve_linux_libc() {
    if [ "${TSPP_OS}" != "linux" ]; then
        TSPP_LIBC=""
        return
    fi

    if command -v ldd >/dev/null 2>&1; then
        local ldd_line
        ldd_line="$(ldd --version 2>&1 | head -n 1 | tr '[:upper:]' '[:lower:]')"

        if printf '%s' "${ldd_line}" | grep -q "musl"; then
            TSPP_LIBC="musl"
            return
        fi

        if printf '%s' "${ldd_line}" | grep -q "gnu"; then
            TSPP_LIBC="gnu"
            return
        fi
    fi

    if command -v getconf >/dev/null 2>&1 && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
        TSPP_LIBC="gnu"
        return
    fi

    TSPP_LIBC="musl"
}

# resolve the rust target triple for release assets
resolve_target_triple() {
    case "${TSPP_OS}:${TSPP_ARCH}:${TSPP_LIBC}" in
        darwin:arm64:*)
            TSPP_TARGET_TRIPLE="aarch64-apple-darwin"
            ;;
        darwin:x64:*)
            TSPP_TARGET_TRIPLE="x86_64-apple-darwin"
            ;;
        linux:arm64:gnu)
            TSPP_TARGET_TRIPLE="aarch64-unknown-linux-gnu"
            ;;
        linux:x64:gnu)
            TSPP_TARGET_TRIPLE="x86_64-unknown-linux-gnu"
            ;;
        linux:*:musl)
            fail "musl targets are not published yet, use npm or build from source"
            ;;
        *)
            fail "unsupported target: ${TSPP_OS}/${TSPP_ARCH}/${TSPP_LIBC}"
            ;;
    esac
}

# resolve the release version and tag
resolve_release_version() {
    if [ "${TSPP_VERSION_INPUT}" = "latest" ]; then
        local release_json
        release_json="$(fetch_json "${TSPP_API_BASE_URL}/releases/latest")"

        local latest_tag
        latest_tag="$(parse_release_tag "${release_json}")"
        if [ -z "${latest_tag}" ]; then
            fail "failed to resolve latest release tag"
        fi

        TSPP_RELEASE_TAG="${latest_tag}"
        TSPP_VERSION_VALUE="${latest_tag#v}"
        return
    fi

    if [[ "${TSPP_VERSION_INPUT}" == v* ]]; then
        TSPP_RELEASE_TAG="${TSPP_VERSION_INPUT}"
        TSPP_VERSION_VALUE="${TSPP_VERSION_INPUT#v}"
        return
    fi

    TSPP_VERSION_VALUE="${TSPP_VERSION_INPUT}"
    TSPP_RELEASE_TAG="v${TSPP_VERSION_VALUE}"
}

# resolve release asset names and urls
resolve_release_assets() {
    TSPP_ARCHIVE_NAME="tspp-${TSPP_VERSION_VALUE}-${TSPP_TARGET_TRIPLE}.tar.gz"
    TSPP_ARCHIVE_URL="${TSPP_RELEASE_BASE_URL}/${TSPP_RELEASE_TAG}/${TSPP_ARCHIVE_NAME}"
    TSPP_CHECKSUMS_URL="${TSPP_RELEASE_BASE_URL}/${TSPP_RELEASE_TAG}/${TSPP_CHECKSUMS_NAME}"
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

# verify archive checksum against sha file
verify_checksum() {
    local archive_path="$1"
    local checksums_path="$2"

    local checksum_line
    checksum_line="$(grep -E "(\\*| )${TSPP_ARCHIVE_NAME}\$" "${checksums_path}" | head -n 1 || true)"
    if [ -z "${checksum_line}" ]; then
        fail "checksum entry not found for ${TSPP_ARCHIVE_NAME}"
    fi

    local expected_hash
    expected_hash="$(printf '%s' "${checksum_line}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"
    local actual_hash
    actual_hash="$(sha256_file "${archive_path}" | tr '[:upper:]' '[:lower:]')"

    if [ "${expected_hash}" != "${actual_hash}" ]; then
        fail "checksum mismatch for ${TSPP_ARCHIVE_NAME}"
    fi
}

# resolve an extracted binary path
find_extracted_binary() {
    local binary_name="$1"
    local extract_directory="$2"

    local discovered_path
    discovered_path="$(find "${extract_directory}" -type f -name "${binary_name}" | head -n 1 || true)"
    if [ -z "${discovered_path}" ]; then
        fail "missing binary in archive: ${binary_name}"
    fi

    printf '%s\n' "${discovered_path}"
}

# install binaries into the target directory
install_binaries() {
    local extract_directory="$1"
    mkdir -p "${TSPP_INSTALL_DIR}"

    local binary_name
    for binary_name in tspp tsppc; do
        local source_binary
        source_binary="$(find_extracted_binary "${binary_name}" "${extract_directory}")"
        cp "${source_binary}" "${TSPP_INSTALL_DIR}/${binary_name}"
        chmod 755 "${TSPP_INSTALL_DIR}/${binary_name}"
    done
}

# resolve the shell rc file path
resolve_rc_file() {
    local shell_name
    shell_name="$(basename "${SHELL:-}")"
    case "${shell_name}" in
        zsh)
            printf '%s\n' "${HOME}/.zshrc"
            ;;
        bash)
            if [ -f "${HOME}/.bashrc" ]; then
                printf '%s\n' "${HOME}/.bashrc"
            else
                printf '%s\n' "${HOME}/.bash_profile"
            fi
            ;;
        *)
            printf '%s\n' "${HOME}/.profile"
            ;;
    esac
}

# update path in shell rc when needed
ensure_path_entry() {
    if [ "${TSPP_NO_MODIFY_PATH}" = "1" ]; then
        info "skipping path update because TSPP_NO_MODIFY_PATH=1"
        return
    fi

    if printf ':%s:' "${PATH}" | grep -q ":${TSPP_INSTALL_DIR}:"; then
        info "install directory already present in current PATH"
        return
    fi

    local rc_file
    rc_file="$(resolve_rc_file)"
    touch "${rc_file}"

    if grep -Fq "${TSPP_INSTALL_DIR}" "${rc_file}"; then
        info "install directory already present in ${rc_file}"
        return
    fi

    printf '\n# tspp cli\nexport PATH="%s:$PATH"\n' "${TSPP_INSTALL_DIR}" >> "${rc_file}"
    info "added ${TSPP_INSTALL_DIR} to PATH in ${rc_file}"
}

# run the installer flow
main() {
    # validate required commands
    require_command tar
    require_command awk
    require_command grep
    require_command sed

    # resolve runtime and release metadata
    resolve_os
    resolve_arch
    resolve_linux_libc
    resolve_target_triple
    resolve_release_version
    resolve_release_assets

    info "installing tspp ${TSPP_VERSION_VALUE} for ${TSPP_TARGET_TRIPLE}"

    # create a temporary workspace
    local temp_directory
    temp_directory="$(mktemp -d)"
    trap 'rm -rf "${temp_directory:-}"' EXIT

    local archive_path="${temp_directory}/${TSPP_ARCHIVE_NAME}"
    local checksums_path="${temp_directory}/${TSPP_CHECKSUMS_NAME}"
    local extract_directory="${temp_directory}/extract"

    # download release files
    download_file "${TSPP_ARCHIVE_URL}" "${archive_path}"
    download_file "${TSPP_CHECKSUMS_URL}" "${checksums_path}"

    # verify and extract archive
    verify_checksum "${archive_path}" "${checksums_path}"
    mkdir -p "${extract_directory}"
    tar -xzf "${archive_path}" -C "${extract_directory}"

    # install binaries and update path
    install_binaries "${extract_directory}"
    ensure_path_entry

    # print final guidance
    info "installed binaries into ${TSPP_INSTALL_DIR}"
    info "run: tspp --version"
}

main "$@"
