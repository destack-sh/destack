#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -gt 0 ]; then
	dry="$1"
else
	dry="--dry-run"
fi

dotnet_bin="$(command -v dotnet 2>/dev/null || true)"
if [ -z "${dotnet_bin}" ] && [ -x "${HOME}/.dotnet/dotnet" ]; then
	dotnet_bin="${HOME}/.dotnet/dotnet"
fi

if [ -z "${dotnet_bin}" ]; then
	echo "missing dotnet command, run: just bridge/ensure-toolchain"
	exit 1
fi

(cd dotnet && "${dotnet_bin}" pack -c Release --nologo src/Destack/Destack.csproj)

if [ "${dry}" = "--dry-run" ]; then
	echo "Dry run: built and packed nuget package"
	exit 0
fi

nuget_api_key="${NUGET_OIDC_API_KEY:-${NUGET_API_KEY:-}}"
if [ -z "${nuget_api_key}" ]; then
	echo "missing nuget publish key, set NUGET_OIDC_API_KEY or NUGET_API_KEY"
	exit 1
fi

version="$(cat ../VERSION.txt)"
nupkg_path="dotnet/src/Destack/bin/Release/Destack.${version}.nupkg"
snupkg_path="dotnet/src/Destack/bin/Release/Destack.${version}.snupkg"
"${dotnet_bin}" nuget push "${nupkg_path}" --source "https://api.nuget.org/v3/index.json" --api-key "${nuget_api_key}" --skip-duplicate
if [ -f "${snupkg_path}" ]; then
	"${dotnet_bin}" nuget push "${snupkg_path}" --source "https://api.nuget.org/v3/index.json" --api-key "${nuget_api_key}" --skip-duplicate
fi
