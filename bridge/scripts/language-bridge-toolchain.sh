#!/usr/bin/env bash

set -euo pipefail

command_path() {
	command -v "$1" 2>/dev/null || true
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

validate_toolchains() {
	require_command cargo "install rust and re-run: just bridge/doctor-toolchain"
	require_command bun "install bun and re-run: just bridge/doctor-toolchain"
	require_command node "install node and re-run: just bridge/doctor-toolchain"
	require_command npm "install npm and re-run: just bridge/doctor-toolchain"
	require_command python3 "install python and re-run: just bridge/doctor-toolchain"

	cargo --version
	bun --version
	node --version
	npm --version
	python3 --version
}

doctor_toolchains() {
	local host_kernel
	local host_arch
	local has_error

	host_kernel="$(uname -s)"
	host_arch="$(uname -m)"
	has_error="0"

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
		else
			print_error "${description}: missing command '${command_name}'"
		fi
	}

	printf 'bridge toolchain doctor\n'
	printf 'host: %s (%s)\n' "${host_kernel}" "${host_arch}"

	check_command cargo "cargo"
	check_command bun "bun"
	check_command node "node"
	check_command npm "npm"
	check_command python3 "python3"

	if [[ "${has_error}" == "1" ]]; then
		printf 'bridge toolchain doctor: failed\n'
		printf 'run: just bridge/install-toolchain\n'
		return 1
	fi

	printf 'bridge toolchain doctor: ok\n'
	return 0
}

ensure_toolchains() {
	if doctor_toolchains; then
		return 0
	fi

	echo "missing required bridge toolchains"
	echo "install the missing tools and re-run: just bridge/doctor-toolchain"
	return 1
}

usage() {
	echo "usage: $0 <doctor|install|ensure>"
}

case "${1:-}" in
doctor)
	doctor_toolchains
	;;
install)
	validate_toolchains
	;;
ensure)
	ensure_toolchains
	;;
*)
	usage
	exit 2
	;;
esac
