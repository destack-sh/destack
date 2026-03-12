#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repository_root="$(cd "${script_directory}/../.." && pwd)"
host_kernel="$(uname -s)"

# shellcheck disable=SC1091
source "${repository_root}/dev/toolchain/versions.sh"

command_path() {
	command -v "$1" 2>/dev/null || true
}

prepare_environment() {
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
}

require_command() {
	local command_name="$1"
	local install_hint="$2"

	if [[ -n "$(command_path "${command_name}")" ]]; then
		return 0
	fi

	echo "missing required command '${command_name}'"
	echo "${install_hint}"
	exit 1
}

install_dotnet_sdk() {
	if [[ -n "$(command_path dotnet)" ]]; then
		return 0
	fi

	dotnet_install_directory="${HOME}/.dotnet"
	dotnet_install_script="$(mktemp)"
	curl -fsSL https://dot.net/v1/dotnet-install.sh -o "${dotnet_install_script}"
	bash "${dotnet_install_script}" --channel "${DESTACK_DOTNET_CHANNEL}" --install-dir "${dotnet_install_directory}"
	rm -f "${dotnet_install_script}"

	export PATH="${dotnet_install_directory}:${PATH}"
	if [[ -n "${GITHUB_PATH:-}" ]]; then
		echo "${dotnet_install_directory}" >>"${GITHUB_PATH}"
	fi
}

install_linux_toolchains() {
	if ! command -v apt-get >/dev/null 2>&1; then
		echo "apt-get is required on linux hosts"
		echo "install language bridge toolchains manually and re-run this script"
		exit 1
	fi

	sudo apt-get update
	sudo apt-get install -y \
		maven \
		"openjdk-${DESTACK_JAVA_MAJOR_VERSION}-jdk" \
		elixir \
		erlang-dev \
		golang-go \
		ruby-full \
		wget \
		gpg \
		curl

	if [[ ! -f /usr/share/keyrings/dart.gpg ]]; then
		wget -qO- https://dl-ssl.google.com/linux/linux_signing_key.pub |
			gpg --dearmor |
			sudo tee /usr/share/keyrings/dart.gpg >/dev/null
	fi

	if [[ ! -f /etc/apt/sources.list.d/dart_stable.list ]]; then
		echo "deb [signed-by=/usr/share/keyrings/dart.gpg] https://storage.googleapis.com/download.dartlang.org/linux/debian stable main" |
			sudo tee /etc/apt/sources.list.d/dart_stable.list >/dev/null
	fi

	if [[ -z "$(command_path dart)" ]]; then
		sudo apt-get update
		sudo apt-get install -y dart
	fi

	install_dotnet_sdk
}

install_darwin_toolchains() {
	if ! command -v brew >/dev/null 2>&1; then
		echo "homebrew is required on darwin hosts"
		echo "install homebrew or install language bridge toolchains manually"
		exit 1
	fi

	brew install "openjdk@${DESTACK_JAVA_MAJOR_VERSION}" maven go ruby elixir shfmt shellcheck || true
	brew tap dart-lang/dart || true
	brew install dart || true

	openjdk_directory="$(brew --prefix "openjdk@${DESTACK_JAVA_MAJOR_VERSION}")"
	if [[ -d "${openjdk_directory}/bin" ]]; then
		export PATH="${openjdk_directory}/bin:${PATH}"
	fi

	install_dotnet_sdk
}

install_toolchains() {
	prepare_environment

	if [[ "${host_kernel}" == "Linux" ]]; then
		install_linux_toolchains
	elif [[ "${host_kernel}" == "Darwin" ]]; then
		install_darwin_toolchains
	else
		echo "unsupported host kernel for language bridge toolchain install: ${host_kernel}"
		exit 1
	fi

	require_command dotnet "install .NET SDK ${DESTACK_DOTNET_CHANNEL} and re-run: just bridge/install-toolchain"
	require_command mvn "install maven and re-run: just bridge/install-toolchain"
	require_command java "install JDK ${DESTACK_JAVA_MAJOR_VERSION} and re-run: just bridge/install-toolchain"
	require_command javac "install JDK ${DESTACK_JAVA_MAJOR_VERSION} and re-run: just bridge/install-toolchain"
	require_command go "install go and re-run: just bridge/install-toolchain"
	require_command ruby "install ruby and re-run: just bridge/install-toolchain"
	require_command dart "install dart and re-run: just bridge/install-toolchain"
	require_command mix "install elixir and re-run: just bridge/install-toolchain"

	if [[ "${host_kernel}" == "Darwin" ]]; then
		require_command swift "install swift and re-run: just bridge/install-toolchain"
	fi

	dotnet --version
	mvn --version
	java --version
	go version
	ruby --version
	dart --version
	mix --version
}

doctor_toolchains() {
	local host_arch
	local has_error

	host_arch="$(uname -m)"
	has_error="0"

	prepare_environment

	print_ok() {
		local message="$1"
		printf '[ok] %s\n' "${message}"
	}

	print_warn() {
		local message="$1"
		printf '[warn] %s\n' "${message}"
	}

	print_error() {
		local message="$1"
		printf '[error] %s\n' "${message}"
		has_error="1"
	}

	check_command() {
		local command_name="$1"
		local required="$2"
		local description="$3"

		local resolved_path
		resolved_path="$(command -v "${command_name}" 2>/dev/null || true)"

		if [[ -n "${resolved_path}" ]]; then
			print_ok "${description}: ${resolved_path}"
			return 0
		fi

		if [[ "${required}" == "required" ]]; then
			print_error "${description}: missing command '${command_name}'"
		else
			print_warn "${description}: missing optional command '${command_name}'"
		fi
	}

	printf 'bridge language toolchain doctor\n'
	printf 'host: %s (%s)\n' "${host_kernel}" "${host_arch}"

	check_command cargo required "cargo"
	check_command bun required "bun"
	check_command node required "node"
	check_command npm required "npm"
	check_command python3 required "python3"
	check_command dotnet required ".NET SDK"
	check_command mvn required "maven"
	check_command java required "java runtime"
	check_command javac required "java compiler"
	check_command go required "go"
	check_command ruby required "ruby"

	if [[ "${host_kernel}" == "Linux" ]]; then
		check_command dart required "dart sdk"
		check_command mix required "elixir"
		check_command swift optional "swift"
	elif [[ "${host_kernel}" == "Darwin" ]]; then
		check_command dart required "dart sdk"
		check_command mix required "elixir"
		check_command swift required "swift"
	else
		check_command dart optional "dart sdk"
		check_command mix optional "elixir"
		check_command swift optional "swift"
	fi

	if [[ "${has_error}" == "1" ]]; then
		printf 'bridge language toolchain doctor: failed\n'
		printf 'run: just bridge/install-toolchain\n'
		printf 'or: DESTACK_AUTO_INSTALL_TOOLCHAINS=1 just bridge/test-language-bridges\n'
		return 1
	fi

	printf 'bridge language toolchain doctor: ok\n'
	return 0
}

ensure_toolchains() {
	local auto_install

	auto_install="${DESTACK_AUTO_INSTALL_TOOLCHAINS:-0}"

	if doctor_toolchains; then
		return 0
	fi

	if [[ "${auto_install}" != "1" ]]; then
		echo "missing required bridge language toolchains"
		echo "run: just bridge/install-toolchain"
		echo "or run with auto install: DESTACK_AUTO_INSTALL_TOOLCHAINS=1 just bridge/test-language-bridges"
		return 1
	fi

	echo "auto install requested: installing bridge language toolchains"
	install_toolchains
	doctor_toolchains
}

usage() {
	echo "usage: $0 <doctor|install|ensure>"
}

command="${1:-}"
case "${command}" in
doctor)
	doctor_toolchains
	;;
install)
	install_toolchains
	;;
ensure)
	ensure_toolchains
	;;
*)
	usage
	exit 1
	;;
esac
