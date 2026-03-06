#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

run_doctor() {
	"${script_directory}/runtime-toolchain-doctor.sh"
}

run_install() {
	"${script_directory}/runtime-toolchain-install.sh"
}

run_lint() {
	"${script_directory}/runtime-toolchain-lint.sh"
}

run_ensure() {
	local auto_install

	auto_install="${DESTACK_AUTO_INSTALL_TOOLCHAINS:-0}"

	if run_doctor; then
		return 0
	fi

	if [ "${auto_install}" != "1" ]; then
		echo "missing required runtime toolchains"
		echo "run: just language/install-toolchain"
		echo "or run with auto install: DESTACK_AUTO_INSTALL_TOOLCHAINS=1 just language/ensure-toolchain"
		return 1
	fi

	echo "auto install requested: installing runtime toolchains"
	run_install
	run_doctor
}

usage() {
	echo "usage: $0 <doctor|install|ensure|lint>"
}

command="${1:-}"
case "${command}" in
doctor)
	run_doctor
	;;
install)
	run_install
	;;
ensure)
	run_ensure
	;;
lint)
	run_lint
	;;
*)
	usage
	exit 1
	;;
esac
