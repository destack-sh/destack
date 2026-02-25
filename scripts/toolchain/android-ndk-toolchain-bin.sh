#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <android-ndk-root>" >&2
    exit 1
fi

ndk_root="$1"
toolchain_prebuilt_root="${ndk_root}/toolchains/llvm/prebuilt"
toolchain_bin=""

case "$(uname -s):$(uname -m)" in
    Linux:*)
        candidates=("linux-x86_64")
        ;;
    Darwin:arm64)
        candidates=("darwin-arm64" "darwin-x86_64")
        ;;
    Darwin:x86_64)
        candidates=("darwin-x86_64" "darwin-arm64")
        ;;
    *)
        candidates=("linux-x86_64" "darwin-arm64" "darwin-x86_64" "windows-x86_64")
        ;;
esac

for candidate in "${candidates[@]}"; do
    candidate_bin="${toolchain_prebuilt_root}/${candidate}/bin"
    if [ -d "${candidate_bin}" ]; then
        toolchain_bin="${candidate_bin}"
        break
    fi
done

if [ -z "${toolchain_bin}" ]; then
    echo "missing android llvm toolchain bin directory under ${toolchain_prebuilt_root}" >&2
    exit 1
fi

echo "${toolchain_bin}"
