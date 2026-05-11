#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bridge_directory="$(cd "${script_directory}/.." && pwd)"
repository_directory="$(cd "${bridge_directory}/.." && pwd)"
host_kernel="$(uname -s)"

cleanup() {
	if [[ -d "${bridge_directory}/.bridge-python-aliases-venv" ]]; then
		rm -r "${bridge_directory}/.bridge-python-aliases-venv"
	fi

	if [[ -d "${bridge_directory}/python/aliases/destack-py/dist" ]]; then
		rm -r "${bridge_directory}/python/aliases/destack-py/dist"
	fi
}

trap cleanup EXIT

require_tool_on_ci() {
	local command_name="$1"
	local package_name="$2"

	if [[ -z "${CI:-}" ]]; then
		return
	fi

	if ! command -v "${command_name}" >/dev/null 2>&1; then
		echo "${package_name} is required for bridge ci checks"
		exit 1
	fi
}

if [[ "${host_kernel}" == "Darwin" ]]; then
	capi_library="${repository_directory}/target/release/libdestack_capi.dylib"
else
	capi_library="${repository_directory}/target/release/libdestack_capi.so"
fi

cd "${bridge_directory}"

require_tool_on_ci bun "Bun"
require_tool_on_ci node "Node.js runtime"

cargo build --release --manifest-path capi/Cargo.toml
cargo check --manifest-path capi/Cargo.toml
DESTACK_CAPI_LIB="${capi_library}" python3 -c 'import ctypes, os; path = os.environ["DESTACK_CAPI_LIB"]; lib = ctypes.CDLL(path); lib.destack_capi_abi_version.restype = ctypes.c_uint32; lib.destack_capi_is_available.restype = ctypes.c_bool; lib.destack_capi_version.restype = ctypes.c_char_p; assert lib.destack_capi_abi_version() > 0; assert lib.destack_capi_is_available() is True; assert lib.destack_capi_version().decode("utf8") != ""'
cargo check --manifest-path rust/Cargo.toml
cargo check --manifest-path rust/aliases/destack-rs/Cargo.toml

cd typescript
bun run build
node -e 'Promise.all([import("./dist/index.js"), import("./dist/napi.js"), import("./dist/wasm.js")]).then(async ([runtimeModule, napiModule, wasmModule]) => { const autoClient = await runtimeModule.createClient(); const napiClient = await napiModule.createNapiClient(); const wasmClient = await wasmModule.createWasmClient(); if (autoClient.backend !== "napi" && autoClient.backend !== "wasm") { throw new Error("invalid auto backend"); } if (napiClient.backend !== "napi") { throw new Error("invalid napi backend"); } if (wasmClient.backend !== "wasm") { throw new Error("invalid wasm backend"); } if (!autoClient.version() || !napiClient.version() || !wasmClient.version()) { throw new Error("missing version"); } }).catch((error) => { console.error(error); process.exit(1); });'
cd "${bridge_directory}"

npm pack --dry-run ./typescript/aliases/destack-js >/dev/null
npm pack --dry-run ./typescript/aliases/destack-ts >/dev/null

python3 -m venv .bridge-python-aliases-venv
.bridge-python-aliases-venv/bin/python -m pip install --quiet --disable-pip-version-check build
cd python/aliases/destack-py && ../../../.bridge-python-aliases-venv/bin/python -m build --sdist --wheel
cd "${bridge_directory}"
PYTHONPATH="python/src:python/aliases/destack-py/src" DESTACK_CAPI_LIB="${capi_library}" python3 -c 'from destack import create_client; from destack_py import create_client as create_alias_client; client = create_client(); alias_client = create_alias_client(); assert client.backend == "python"; assert alias_client.backend == "python"; assert client.version() != ""; assert client.capi_abi_version() > 0; assert client.capi_is_available() is True'
