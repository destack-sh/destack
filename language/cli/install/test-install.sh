#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# test configuration
DESTACK_VERSION_INPUT="${1:-}"
DESTACK_RELEASE_TAG_INPUT="${DESTACK_RELEASE_TAG:-}"
DESTACK_SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
DESTACK_CLI_DIRECTORY="$(cd -- "${DESTACK_SCRIPT_DIRECTORY}/.." >/dev/null 2>&1 && pwd)"
DESTACK_ARTIFACTS_DIRECTORY="${2:-${DESTACK_CLI_DIRECTORY}/install/artifacts}"
DESTACK_INSTALL_SCRIPT_PATH="${DESTACK_SCRIPT_DIRECTORY}/install.sh"

# print an error message and exit
fail() {
    printf '%s\n' "error: $*" >&2
    exit 1
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

# resolve linux libc family
resolve_linux_libc() {
    if command -v ldd >/dev/null 2>&1; then
        local ldd_line
        ldd_line="$(ldd --version 2>&1 | head -n 1 | tr '[:upper:]' '[:lower:]')"

        if printf '%s' "${ldd_line}" | grep -q "gnu"; then
            printf '%s\n' "gnu"
            return
        fi

        if printf '%s' "${ldd_line}" | grep -q "musl"; then
            printf '%s\n' "musl"
            return
        fi
    fi

    if command -v getconf >/dev/null 2>&1 && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
        printf '%s\n' "gnu"
        return
    fi

    printf '%s\n' "musl"
}

# resolve the current unix release target triple
resolve_unix_target_triple() {
    local os_name
    os_name="$(uname -s)"
    local architecture_name
    architecture_name="$(uname -m)"

    case "${os_name}:${architecture_name}" in
        Darwin:arm64|Darwin:aarch64)
            printf '%s\n' "aarch64-apple-darwin"
            return
            ;;
        Darwin:x86_64|Darwin:amd64)
            printf '%s\n' "x86_64-apple-darwin"
            return
            ;;
        Linux:arm64|Linux:aarch64)
            local libc_family
            libc_family="$(resolve_linux_libc)"
            if [ "${libc_family}" = "gnu" ]; then
                printf '%s\n' "aarch64-unknown-linux-gnu"
                return
            fi
            ;;
        Linux:x86_64|Linux:amd64)
            local libc_family
            libc_family="$(resolve_linux_libc)"
            if [ "${libc_family}" = "gnu" ]; then
                printf '%s\n' "x86_64-unknown-linux-gnu"
                return
            fi
            ;;
    esac

    fail "unsupported test host target: ${os_name}/${architecture_name}"
}

# run an installer smoke test against local artifacts
main() {
    local version_value
    version_value="$(resolve_version)"
    local release_tag
    release_tag="$(resolve_release_tag "${version_value}")"
    local target_triple
    target_triple="$(resolve_unix_target_triple)"
    local temp_directory
    temp_directory="$(mktemp -d)"
    trap 'rm -rf "${temp_directory:-}"' EXIT

    local release_directory="${temp_directory}/releases/download/${release_tag}"
    local install_directory="${temp_directory}/install/bin"
    mkdir -p "${release_directory}"

    cp "${DESTACK_ARTIFACTS_DIRECTORY}/destack-${version_value}-${target_triple}.tar.gz" "${release_directory}/"
    cp "${DESTACK_ARTIFACTS_DIRECTORY}/SHA256SUMS" "${release_directory}/"

    DESTACK_VERSION="${version_value}" \
    DESTACK_RELEASE_BASE_URL="file://${temp_directory}/releases/download" \
    DESTACK_INSTALL="${install_directory}" \
    DESTACK_NO_MODIFY_PATH=1 \
        bash "${DESTACK_INSTALL_SCRIPT_PATH}"

    "${install_directory}/destack" --version >/dev/null
    "${install_directory}/ds" --version >/dev/null
}

main "$@"
