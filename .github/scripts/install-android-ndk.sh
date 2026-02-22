#!/usr/bin/env bash
set -euo pipefail

sdk_root="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/.android-sdk}}"
cmdline_tools_version="${ANDROID_CMDLINE_TOOLS_VERSION:-13114758}"
ndk_version="${ANDROID_NDK_VERSION:-27.2.12479018}"
api_level="${ANDROID_API_LEVEL:-24}"
host_os="$(uname -s)"
host_arch="$(uname -m)"

cmdline_tools_directory="${sdk_root}/cmdline-tools"
latest_directory="${cmdline_tools_directory}/latest"
sdkmanager="${latest_directory}/bin/sdkmanager"
archive_platform=""
if [ "${host_os}" = "Linux" ]; then
    archive_platform="linux"
elif [ "${host_os}" = "Darwin" ]; then
    archive_platform="mac"
else
    echo "unsupported host os for android commandline tools: ${host_os}" >&2
    exit 1
fi

archive_name="commandlinetools-${archive_platform}-${cmdline_tools_version}_latest.zip"
archive_url="https://dl.google.com/android/repository/${archive_name}"

mkdir -p "${cmdline_tools_directory}" "${sdk_root}/licenses"

if [ ! -x "${sdkmanager}" ]; then
    temporary_directory="$(mktemp -d)"
    archive_path="${temporary_directory}/${archive_name}"

    curl -fsSL "${archive_url}" -o "${archive_path}"
    unzip -q "${archive_path}" -d "${temporary_directory}"

    rm -rf "${latest_directory}"
    mkdir -p "${latest_directory}"
    mv "${temporary_directory}/cmdline-tools/"* "${latest_directory}/"
    rm -rf "${temporary_directory}"
fi

# sdkmanager closes stdin once licenses are accepted:
# with pipefail enabled this can surface yes SIGPIPE as a false failure
set +o pipefail
yes | "${sdkmanager}" --sdk_root="${sdk_root}" --licenses >/dev/null
set -o pipefail

"${sdkmanager}" --sdk_root="${sdk_root}" \
    "platforms;android-${api_level}" \
    "ndk;${ndk_version}" >/dev/null

toolchain_prebuilt_root="${sdk_root}/ndk/${ndk_version}/toolchains/llvm/prebuilt"
toolchain_bin=""

if [ "${host_os}" = "Linux" ]; then
    candidates=("linux-x86_64")
elif [ "${host_os}" = "Darwin" ] && [ "${host_arch}" = "arm64" ]; then
    candidates=("darwin-arm64" "darwin-x86_64")
elif [ "${host_os}" = "Darwin" ] && [ "${host_arch}" = "x86_64" ]; then
    candidates=("darwin-x86_64" "darwin-arm64")
else
    candidates=("linux-x86_64" "darwin-arm64" "darwin-x86_64")
fi

for candidate in "${candidates[@]}"; do
    candidate_bin="${toolchain_prebuilt_root}/${candidate}/bin"
    if [ -d "${candidate_bin}" ]; then
        toolchain_bin="${candidate_bin}"
        break
    fi
done

if [ -z "${toolchain_bin}" ]; then
    echo "missing ndk prebuilt toolchain under ${toolchain_prebuilt_root}" >&2
    exit 1
fi

if [ ! -x "${toolchain_bin}/aarch64-linux-android${api_level}-clang" ]; then
    echo "missing android clang toolchain wrapper in ${toolchain_bin}" >&2
    exit 1
fi

echo "android sdk root: ${sdk_root}"
echo "android ndk root: ${sdk_root}/ndk/${ndk_version}"
