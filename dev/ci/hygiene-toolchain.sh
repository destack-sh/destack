#!/usr/bin/env bash

set -euo pipefail

host_kernel="$(uname -s)"

command_path() {
	command -v "$1" 2>/dev/null || true
}

prepare_environment() {
	if [[ -x "${HOME}/.local/bin/actionlint" ]]; then
		export PATH="${HOME}/.local/bin:${PATH}"
	fi
}

doctor_toolchains() {
	local has_error
	has_error="0"

	prepare_environment

	print_ok() {
		local message="$1"
		printf '[ok] %s\n' "${message}"
	}

	print_error() {
		local message="$1"
		printf '[error] %s\n' "${message}"
		has_error="1"
	}

	check_command() {
		local command_name="$1"
		local description="$2"

		local resolved_path
		resolved_path="$(command -v "${command_name}" 2>/dev/null || true)"
		if [[ -n "${resolved_path}" ]]; then
			print_ok "${description}: ${resolved_path}"
			return 0
		fi

		print_error "${description}: missing command '${command_name}'"
		return 1
	}

	printf 'ci hygiene toolchain doctor\n'
	printf 'host: %s\n' "${host_kernel}"

	check_command actionlint "actionlint"
	check_command shellcheck "shellcheck"
	check_command shfmt "shfmt"

	if [[ "${has_error}" == "1" ]]; then
		printf 'ci hygiene toolchain doctor: failed\n'
		printf 'run: just install-hygiene-toolchain\n'
		printf 'or: DESTACK_AUTO_INSTALL_TOOLCHAINS=1 just check-hygiene\n'
		return 1
	fi

	printf 'ci hygiene toolchain doctor: ok\n'
	return 0
}

install_actionlint_binary() {
	local install_directory="${HOME}/.local/bin"
	local system_directory="/usr/local/bin"
	local system_target="${system_directory}/actionlint"
	local installer_script
	local user_target

	user_target="${install_directory}/actionlint"
	if [[ -n "$(command_path actionlint)" ]]; then
		return 0
	fi

	mkdir -p "${install_directory}"

	installer_script="$(mktemp)"
	curl -fsSL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash -o "${installer_script}"
	bash "${installer_script}" latest "${install_directory}"
	rm -f "${installer_script}"

	if [[ -w "${system_directory}" ]]; then
		cp "${user_target}" "${system_target}"
	elif command -v sudo >/dev/null 2>&1; then
		sudo cp "${user_target}" "${system_target}"
	fi

	if [[ -x "${system_target}" ]]; then
		export PATH="${system_directory}:${PATH}"
	fi

	export PATH="${install_directory}:${PATH}"
	if [[ -n "${GITHUB_PATH:-}" ]]; then
		echo "${install_directory}" >>"${GITHUB_PATH}"
	fi
}

install_toolchains() {
	prepare_environment

	if [[ "${host_kernel}" == "Linux" ]]; then
		if ! command -v apt-get >/dev/null 2>&1; then
			echo "apt-get is required on linux hosts"
			echo "install actionlint, shellcheck, and shfmt manually"
			exit 1
		fi

		sudo apt-get update
		sudo apt-get install -y shellcheck shfmt curl tar gzip
		install_actionlint_binary
	elif [[ "${host_kernel}" == "Darwin" ]]; then
		if ! command -v brew >/dev/null 2>&1; then
			echo "homebrew is required on darwin hosts"
			echo "install actionlint, shellcheck, and shfmt manually"
			exit 1
		fi

		brew install actionlint shellcheck shfmt
	else
		echo "unsupported host kernel for ci hygiene toolchain install: ${host_kernel}"
		exit 1
	fi

	actionlint -version
	shellcheck --version
	shfmt --version
}

ensure_toolchains() {
	local auto_install
	auto_install="${DESTACK_AUTO_INSTALL_TOOLCHAINS:-0}"

	if doctor_toolchains; then
		return 0
	fi

	if [[ "${auto_install}" != "1" ]]; then
		echo "missing required ci hygiene toolchains"
		echo "run: just install-hygiene-toolchain"
		echo "or run with auto install: DESTACK_AUTO_INSTALL_TOOLCHAINS=1 just check-hygiene"
		return 1
	fi

	echo "auto install requested: installing ci hygiene toolchains"
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
