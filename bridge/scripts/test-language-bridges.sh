#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bridge_directory="$(cd "${script_directory}/.." && pwd)"
repository_directory="$(cd "${bridge_directory}/.." && pwd)"
host_kernel="$(uname -s)"

# shellcheck disable=SC1091
source "${repository_directory}/scripts/toolchain/versions.sh"

if [[ -x "${HOME}/.dotnet/dotnet" ]]; then
	export PATH="${HOME}/.dotnet:${PATH}"
fi

if [[ "${host_kernel}" == "Darwin" ]] && command -v brew >/dev/null 2>&1; then
	openjdk_directory="$(brew --prefix "openjdk@${DESTACK_JAVA_MAJOR_VERSION}" 2>/dev/null || true)"
	if [[ -d "${openjdk_directory}/bin" ]]; then
		export PATH="${openjdk_directory}/bin:${PATH}"
	fi
	if [[ -d "${openjdk_directory}/libexec/openjdk.jdk/Contents/Home" ]]; then
		export JAVA_HOME="${openjdk_directory}/libexec/openjdk.jdk/Contents/Home"
	fi
fi

cleanup() {
	if [[ -d "${bridge_directory}/.bridge-dotnet-smoke" ]]; then
		rm -r "${bridge_directory}/.bridge-dotnet-smoke"
	fi

	if [[ -f "${bridge_directory}/dart/.bridge-smoke.dart" ]]; then
		rm -f "${bridge_directory}/dart/.bridge-smoke.dart"
	fi

	if [[ -d "${bridge_directory}/.bridge-python-compat-venv" ]]; then
		rm -r "${bridge_directory}/.bridge-python-compat-venv"
	fi

	if [[ -d "${bridge_directory}/python/compat/destack-py/dist" ]]; then
		rm -r "${bridge_directory}/python/compat/destack-py/dist"
	fi

	if [[ -f "${bridge_directory}/ruby/ext/destack_ext/Makefile" ]]; then
		make -C "${bridge_directory}/ruby/ext/destack_ext" clean >/dev/null || true
		rm -f "${bridge_directory}/ruby/ext/destack_ext/Makefile"
	fi

	if command -v swift >/dev/null 2>&1; then
		(cd "${bridge_directory}/swift" && swift package reset)
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

require_tool_on_ci dotnet ".NET SDK"
require_tool_on_ci mvn "Maven"
require_tool_on_ci java "Java runtime"
require_tool_on_ci javac "Java compiler"
require_tool_on_ci bun "Bun"
require_tool_on_ci node "Node.js runtime"
require_tool_on_ci go "Go"
require_tool_on_ci ruby "Ruby"
if [[ "${host_kernel}" == "Linux" ]]; then
	require_tool_on_ci dart "Dart SDK"
	require_tool_on_ci mix "Elixir"
fi

cargo build --release --manifest-path capi/Cargo.toml
cargo check --manifest-path capi/Cargo.toml
DESTACK_CAPI_LIB="${capi_library}" python3 -c 'import ctypes, os; path = os.environ["DESTACK_CAPI_LIB"]; lib = ctypes.CDLL(path); lib.destack_capi_abi_version.restype = ctypes.c_uint32; lib.destack_capi_is_available.restype = ctypes.c_bool; lib.destack_capi_version.restype = ctypes.c_char_p; assert lib.destack_capi_abi_version() > 0; assert lib.destack_capi_is_available() is True; assert lib.destack_capi_version().decode("utf8") != ""'
cargo check --manifest-path rust/Cargo.toml
cargo check --manifest-path rust/compat/destack-rs/Cargo.toml

cd typescript
bun run build
node -e 'Promise.all([import("./dist/index.js"), import("./dist/napi.js"), import("./dist/wasm.js")]).then(async ([runtimeModule, napiModule, wasmModule]) => { const autoClient = await runtimeModule.createClient(); const napiClient = await napiModule.createNapiClient(); const wasmClient = await wasmModule.createWasmClient(); if (autoClient.backend !== "napi" && autoClient.backend !== "wasm") { throw new Error("invalid auto backend"); } if (napiClient.backend !== "napi") { throw new Error("invalid napi backend"); } if (wasmClient.backend !== "wasm") { throw new Error("invalid wasm backend"); } if (!autoClient.version() || !napiClient.version() || !wasmClient.version()) { throw new Error("missing version"); } }).catch((error) => { console.error(error); process.exit(1); });'
cd "${bridge_directory}"

npm pack --dry-run ./typescript/compat/destack-js >/dev/null
npm pack --dry-run ./typescript/compat/destack-ts >/dev/null

python3 -m venv .bridge-python-compat-venv
.bridge-python-compat-venv/bin/python -m pip install --quiet --disable-pip-version-check build
cd python/compat/destack-py && ../../../.bridge-python-compat-venv/bin/python -m build --sdist --wheel
cd "${bridge_directory}"
PYTHONPATH="python/src:python/compat/destack-py/src" DESTACK_CAPI_LIB="${capi_library}" python3 -c 'from destack import create_client; from destack_py import create_client as create_compat_client; client = create_client(); compat_client = create_compat_client(); assert client.backend == "python"; assert compat_client.backend == "python"; assert client.version() != ""; assert client.capi_abi_version() > 0; assert client.capi_is_available() is True'

if command -v dotnet >/dev/null 2>&1; then
	cd dotnet/src/Destack
	dotnet build -c Release --nologo
	cd "${bridge_directory}"

	mkdir -p .bridge-dotnet-smoke
	cat >.bridge-dotnet-smoke/Smoke.csproj <<'EOF'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="../dotnet/src/Destack/Destack.csproj" />
  </ItemGroup>
</Project>
EOF
	cat >.bridge-dotnet-smoke/Program.cs <<'EOF'
using Destack;

var client = new Client();
if (client.Backend != BackendMarker.Value) {
    throw new Exception("backend mismatch");
}
if (string.IsNullOrWhiteSpace(client.Version())) {
    throw new Exception("missing version");
}
if (client.CapiAbiVersion() <= 0) {
    throw new Exception("missing capi abi version");
}
if (!client.CapiIsAvailable()) {
    throw new Exception("capi unavailable");
}
EOF
	DESTACK_CAPI_LIB="${capi_library}" dotnet run -c Release --nologo --project .bridge-dotnet-smoke/Smoke.csproj
else
	echo "dotnet not installed, skipping dotnet bridge checks"
fi

if command -v mvn >/dev/null 2>&1; then
	cd java
	mvn -q -DskipTests package
	mvn -q -DskipTests -DincludeScope=runtime dependency:build-classpath -Dmdep.outputFile=target/runtime.classpath
	runtime_classpath="$(cat target/runtime.classpath)"
	mkdir -p target/smoke
	cat >target/smoke/BridgeSmoke.java <<'EOF'
import com.symbol.destack.Client;

public final class BridgeSmoke {
    public static void main(String[] args) {
        Client client = new Client();
        if (!"java".equals(client.backend())) {
            throw new RuntimeException("backend mismatch");
        }
        if (client.version() == null || client.version().isBlank()) {
            throw new RuntimeException("missing version");
        }
        if (client.capiAbiVersion() <= 0) {
            throw new RuntimeException("missing capi abi version");
        }
        if (!client.capiIsAvailable()) {
            throw new RuntimeException("capi unavailable");
        }
    }
}
EOF
	javac -cp "target/classes:${runtime_classpath}" -d target/smoke target/smoke/BridgeSmoke.java
	DESTACK_CAPI_LIB="${capi_library}" java -cp "target/smoke:target/classes:${runtime_classpath}" BridgeSmoke
	cd "${bridge_directory}"
else
	echo "maven not installed, skipping java bridge checks"
fi

if command -v go >/dev/null 2>&1; then
	cd go
	go test ./...
	GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build ./...
	cd "${bridge_directory}"
else
	echo "go not installed, skipping go bridge checks"
fi

if command -v ruby >/dev/null 2>&1; then
	cd ruby/ext/destack_ext
	ruby extconf.rb >/dev/null
	make >/dev/null
	cd ../..
	ruby -c lib/destack.rb
	ruby -c lib/destack/client.rb
	ruby -c lib/destack/version.rb
	DESTACK_CAPI_LIB="${capi_library}" ruby -Ilib:ext/destack_ext -e 'require "destack"; client = Destack::Client.new; abort("missing capi") if client.capi_abi_version <= 0; abort("capi unavailable") unless client.capi_is_available'
	cd "${bridge_directory}"
else
	echo "ruby not installed, skipping ruby bridge checks"
fi

if command -v dart >/dev/null 2>&1; then
	cd dart
	dart pub get
	dart analyze
	cat >.bridge-smoke.dart <<'EOF'
import "package:destack/destack.dart";

void main() {
  final client = DestackClient();
  if (client.backend != "dart") {
    throw StateError("backend mismatch");
  }
  if (client.version().isEmpty) {
    throw StateError("missing version");
  }
  if (client.capiAbiVersion() <= 0) {
    throw StateError("missing capi abi version");
  }
  if (!client.capiIsAvailable()) {
    throw StateError("capi unavailable");
  }
}
EOF
	DESTACK_CAPI_LIB="${capi_library}" dart run .bridge-smoke.dart
	cd "${bridge_directory}"
else
	echo "dart not installed, skipping dart bridge checks"
fi

if command -v mix >/dev/null 2>&1; then
	cd elixir
	chmod +x scripts/build_nif.sh
	DESTACK_CAPI_LIB="${capi_library}" ./scripts/build_nif.sh
	DESTACK_CAPI_LIB="${capi_library}" mix compile
	DESTACK_CAPI_LIB="${capi_library}" mix run -e 'client = Destack.Client.new(); if Destack.Client.capi_abi_version(client) <= 0, do: raise("missing capi"); unless Destack.Client.capi_is_available(client), do: raise("capi unavailable")'
	cd "${bridge_directory}"
else
	echo "mix not installed, skipping elixir bridge checks"
fi

if command -v swift >/dev/null 2>&1; then
	cd swift
	swift build
	cd "${bridge_directory}"
else
	echo "swift not installed, skipping swift bridge checks"
fi
