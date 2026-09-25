#!/usr/bin/env bash
set -euo pipefail

# stable locale for predictable tool output
export LC_ALL="C"

# validation configuration
DESTACK_SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
DESTACK_CLI_DIRECTORY="$(cd -- "${DESTACK_SCRIPT_DIRECTORY}/.." >/dev/null 2>&1 && pwd)"
DESTACK_VERSION_INPUT="${1:-}"
DESTACK_ARTIFACTS_DIRECTORY="${2:-${DESTACK_CLI_DIRECTORY}/install/artifacts}"
DESTACK_TARGETS_INPUT="${DESTACK_RELEASE_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc}"
DESTACK_RELEASE_TAG_INPUT="${DESTACK_RELEASE_TAG:-}"
DESTACK_RELEASE_CHANNEL_INPUT="${DESTACK_RELEASE_CHANNEL:-release}"
DESTACK_RELEASE_STABILITY_INPUT="${DESTACK_RELEASE_STABILITY:-}"
DESTACK_CHECKSUMS_NAME="SHA256SUMS"
DESTACK_MANIFEST_NAME="manifest.json"
read -r -a DESTACK_TARGETS <<< "${DESTACK_TARGETS_INPUT}"

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
    if [ -z "${stability_value}" ] && [ -f "platform/release/config.json" ]; then
        stability_value="$(node -p 'JSON.parse(require("node:fs").readFileSync("platform/release/config.json", "utf8")).stability')"
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
    local release_tag
    release_tag="$(resolve_release_tag "${version_value}")"
    local release_stability
    release_stability="$(resolve_release_stability)"
    local release_channel
    release_channel="$(resolve_release_channel)"
    local checksums_path="${DESTACK_ARTIFACTS_DIRECTORY}/${DESTACK_CHECKSUMS_NAME}"
    if [ ! -f "${checksums_path}" ]; then
        fail "missing checksums file: ${checksums_path}"
    fi

    local target_triple
    for target_triple in "${DESTACK_TARGETS[@]}"; do
        validate_target_archive "${version_value}" "${target_triple}" "${checksums_path}"
    done

    local manifest_path="${DESTACK_ARTIFACTS_DIRECTORY}/${DESTACK_MANIFEST_NAME}"
    if [ ! -f "${manifest_path}" ]; then
        fail "missing manifest file: ${manifest_path}"
    fi

    if ! command -v python3 >/dev/null 2>&1; then
        fail "python3 is required to validate ${DESTACK_MANIFEST_NAME}"
    fi

    python3 - "${manifest_path}" "${checksums_path}" "${version_value}" "${release_tag}" "${release_channel}" "${release_stability}" "${DESTACK_TARGETS[@]}" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
checksums_path = pathlib.Path(sys.argv[2])
version_value = sys.argv[3]
release_tag = sys.argv[4]
release_channel = sys.argv[5]
release_stability = sys.argv[6]
targets = sys.argv[7:]

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

if manifest.get("version") != version_value:
    raise SystemExit(f"error: manifest version mismatch: {manifest.get('version')} != {version_value}")

if manifest.get("releaseTag") != release_tag:
    raise SystemExit(
        f"error: manifest releaseTag mismatch: {manifest.get('releaseTag')} != {release_tag}"
    )

if manifest.get("channel") != release_channel:
    raise SystemExit(
        f"error: manifest channel mismatch: {manifest.get('channel')} != {release_channel}"
    )

if manifest.get("stability") != release_stability:
    raise SystemExit(
        f"error: manifest stability mismatch: {manifest.get('stability')} != {release_stability}"
    )

if manifest.get("checksumsFile") != "SHA256SUMS":
    raise SystemExit(
        f"error: manifest checksumsFile mismatch: {manifest.get('checksumsFile')} != SHA256SUMS"
    )

assets = manifest.get("assets")
if not isinstance(assets, list):
    raise SystemExit("error: manifest assets must be a list")

assets_by_target = {}
for asset in assets:
    target = asset.get("targetTriple")
    if not isinstance(target, str):
        raise SystemExit("error: manifest asset missing targetTriple")
    assets_by_target[target] = asset

checksum_map = {}
for line in checksums_path.read_text(encoding="utf-8").splitlines():
    line = line.strip()
    if not line:
        continue
    parts = line.split()
    if len(parts) < 2:
        raise SystemExit(f"error: invalid checksum line: {line}")
    checksum_map[parts[-1]] = parts[0].lower()

for target in targets:
    asset = assets_by_target.get(target)
    if asset is None:
        raise SystemExit(f"error: manifest missing target asset: {target}")

    archive_name = asset.get("archiveName")
    archive_sha256 = asset.get("archiveSha256")
    binaries = asset.get("binaries")
    if not isinstance(archive_name, str) or not archive_name:
        raise SystemExit(f"error: manifest archiveName missing for {target}")
    if not isinstance(archive_sha256, str) or not archive_sha256:
        raise SystemExit(f"error: manifest archiveSha256 missing for {target}")
    if binaries != ["destack", "tspp", "tsppc"]:
        raise SystemExit(f"error: manifest binaries mismatch for {target}: {binaries}")

    checksum_value = checksum_map.get(archive_name)
    if checksum_value is None:
        raise SystemExit(f"error: checksum entry missing for {archive_name}")
    if checksum_value != archive_sha256.lower():
        raise SystemExit(
            f"error: manifest checksum mismatch for {archive_name}: {archive_sha256} != {checksum_value}"
        )
PY
}

main "$@"
