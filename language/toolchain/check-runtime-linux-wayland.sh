#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

if [ "$(runtime_host_kernel)" != "Linux" ]; then
	echo "linux wayland runtime checks must run on a linux host"
	exit 1
fi

ensure_weston() {
	runtime_require_or_auto_install_command \
		weston \
		weston \
		"missing weston: install weston to run the runtime wayland lane" \
		"just language/check-runtime-linux-wayland"
}

ensure_weston || exit 1

runtime_test_filter="platform::display::tests::backend::test_display_wayland_capabilities_match_implemented_contract"
runtime_socket_name="destack-wayland-ci"
runtime_startup_timeout_seconds="${DESTACK_WAYLAND_STARTUP_TIMEOUT_SECONDS:-20}"

runtime_directory="$(mktemp -d)"
weston_log="$(mktemp)"
weston_pid=""

cleanup() {
	if [ -n "${weston_pid}" ]; then
		kill "${weston_pid}" >/dev/null 2>&1 || true
		wait "${weston_pid}" >/dev/null 2>&1 || true
	fi

	rm -rf "${runtime_directory}"
	rm -f "${weston_log}"
}

trap cleanup EXIT

export XDG_RUNTIME_DIR="${runtime_directory}"
export WAYLAND_DISPLAY="${runtime_socket_name}"

# start one weston headless compositor and wait for socket readiness
start_weston() {
	weston --backend=headless-backend.so --socket="${WAYLAND_DISPLAY}" --idle-time=0 --log="${weston_log}" >/dev/null 2>&1 &
	weston_pid="$!"
}

wait_for_socket() {
	local remaining_seconds="${runtime_startup_timeout_seconds}"
	while [ "${remaining_seconds}" -gt 0 ]; do
		if [ -S "${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY}" ]; then
			return 0
		fi

		if [ -n "${weston_pid}" ] && ! kill -0 "${weston_pid}" >/dev/null 2>&1; then
			return 1
		fi

		sleep 1
		remaining_seconds="$((remaining_seconds - 1))"
	done

	return 1
}

start_weston
if ! wait_for_socket; then
	echo "weston failed to start headless wayland compositor"
	cat "${weston_log}" || true
	exit 1
fi

cargo test -p tspp_runtime "${runtime_test_filter}" -- --nocapture
