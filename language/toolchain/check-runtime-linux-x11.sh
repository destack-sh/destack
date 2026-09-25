#!/usr/bin/env bash
set -euo pipefail

script_directory="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_directory}/../.." && pwd)"

# shellcheck source=./language/toolchain/lib/runtime-common.sh
source "${script_directory}/lib/runtime-common.sh"

runtime_set_standard_environment

runtime_test_filter="${TSPP_X11_RUNTIME_TEST_FILTER:-platform::display::tests::backend::test_display_x11_capabilities_match_implemented_contract}"

if [ "$(runtime_host_kernel)" != "Linux" ]; then
	container_engine="$(runtime_host_container_engine)"
	if [ -z "${container_engine}" ]; then
		echo "missing container runtime: install docker or podman to run the local linux x11 lane"
		exit 1
	fi

	if ! runtime_host_container_ready "${container_engine}"; then
		echo "linux x11 runtime lane requires one running ${container_engine} daemon or machine"
		exit 1
	fi

	toolchain_channel="$(runtime_rust_toolchain_channel "${repo_root}")"
	if [ -z "${toolchain_channel}" ]; then
		echo "failed to determine rust toolchain channel from rust-toolchain.toml"
		exit 1
	fi

	runtime_run_linux_container_x11_lane "${container_engine}" "${repo_root}" "${toolchain_channel}" "${runtime_test_filter}"
	exit 0
fi

ensure_xvfb() {
	runtime_require_or_auto_install_command \
		Xvfb \
		xvfb \
		"missing Xvfb: install xvfb to run the runtime x11 lane" \
		"just language/check-runtime-linux-x11"
}

ensure_xvfb || exit 1

runtime_display="${TSPP_X11_DISPLAY:-:98}"
runtime_screen="${TSPP_X11_SCREEN:-0}"
runtime_geometry="${TSPP_X11_GEOMETRY:-1280x720x24}"
runtime_startup_timeout_seconds="${TSPP_X11_STARTUP_TIMEOUT_SECONDS:-10}"

xvfb_log="$(mktemp)"
xvfb_pid=""

cleanup() {
	if [ -n "${xvfb_pid}" ]; then
		kill "${xvfb_pid}" >/dev/null 2>&1 || true
		wait "${xvfb_pid}" >/dev/null 2>&1 || true
	fi

	rm -f "${xvfb_log}"
}

trap cleanup EXIT

display_number="${runtime_display#:}"
display_number="${display_number%%.*}"
socket_path="/tmp/.X11-unix/X${display_number}"

# start one headless x11 server and wait until the unix socket appears
start_xvfb() {
	Xvfb "${runtime_display}" -screen "${runtime_screen}" "${runtime_geometry}" -nolisten tcp >"${xvfb_log}" 2>&1 &
	xvfb_pid="$!"
}

wait_for_socket() {
	local remaining_seconds="${runtime_startup_timeout_seconds}"
	while [ "${remaining_seconds}" -gt 0 ]; do
		if [ -S "${socket_path}" ]; then
			return 0
		fi

		if [ -n "${xvfb_pid}" ] && ! kill -0 "${xvfb_pid}" >/dev/null 2>&1; then
			return 1
		fi

		sleep 1
		remaining_seconds="$((remaining_seconds - 1))"
	done

	return 1
}

start_xvfb
if ! wait_for_socket; then
	echo "Xvfb failed to start headless x11 server"
	cat "${xvfb_log}" || true
	exit 1
fi

export DISPLAY="${runtime_display}"

cargo test -p tspp_runtime "${runtime_test_filter}" -- --nocapture
