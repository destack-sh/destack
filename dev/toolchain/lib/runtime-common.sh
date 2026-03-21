#!/usr/bin/env bash

runtime_host_kernel() {
	uname -s
}

runtime_command_path() {
	command_name="$1"

	command -v "${command_name}" 2>/dev/null || true
}

runtime_brew_prefix() {
	if ! command -v brew >/dev/null 2>&1; then
		return 1
	fi

	brew --prefix "$1" 2>/dev/null
}

runtime_java_path() {
	command_path="$(runtime_command_path java)"
	if [ -n "${command_path}" ] && "${command_path}" -version >/dev/null 2>&1; then
		printf '%s\n' "${command_path}"
		return 0
	fi

	if [ "$(runtime_host_kernel)" != "Darwin" ]; then
		return 1
	fi

	java_prefix="$(runtime_brew_prefix openjdk@17 || true)"
	if [ -n "${java_prefix}" ] && [ -x "${java_prefix}/bin/java" ]; then
		printf '%s\n' "${java_prefix}/bin/java"
		return 0
	fi

	return 1
}

runtime_require_command() {
	command_name="$1"
	message="$2"

	if [ -n "$(runtime_command_path "${command_name}")" ]; then
		return 0
	fi

	echo "${message}"
	return 1
}

runtime_rust_target_installed() {
	target="$1"

	if ! runtime_require_command rustup "missing rustup: install rustup to manage rust targets" >/dev/null; then
		return 1
	fi

	rustup target list --installed | grep -Fx "${target}" >/dev/null 2>&1
}

runtime_ensure_rust_target() {
	target="$1"

	if [ -n "$(runtime_command_path rustup)" ]; then
		rustup target add "${target}" >/dev/null
	fi
}

runtime_set_standard_environment() {
	export LC_ALL=C
	export LANG=C
	export LC_CTYPE=C
	export CARGO_INCREMENTAL=0
}

runtime_host_container_engine() {
	if command -v docker >/dev/null 2>&1; then
		printf '%s\n' docker
		return 0
	fi

	if command -v podman >/dev/null 2>&1; then
		printf '%s\n' podman
		return 0
	fi

	printf '%s\n' ""
}

runtime_host_container_ready() {
	container_engine="$1"

	case "${container_engine}" in
	docker)
		docker info >/dev/null 2>&1
		;;
	podman)
		podman info >/dev/null 2>&1
		;;
	*)
		return 1
		;;
	esac
}

runtime_rust_toolchain_channel() {
	repo_root="$1"

	sed -n 's/^channel = "\(.*\)"/\1/p' "${repo_root}/rust-toolchain.toml" | head -n 1
}

runtime_run_full_runtime_crate_lane() {
	cargo_bin="${1:-cargo}"

	LC_ALL=C \
		LANG=C \
		LC_CTYPE=C \
		CARGO_INCREMENTAL=0 \
		"${cargo_bin}" check -p destack_runtime

	LC_ALL=C \
		LANG=C \
		LC_CTYPE=C \
		CARGO_INCREMENTAL=0 \
		"${cargo_bin}" clippy -p destack_runtime --all-targets -- -D warnings

	LC_ALL=C \
		LANG=C \
		LC_CTYPE=C \
		CARGO_INCREMENTAL=0 \
		"${cargo_bin}" test -p destack_runtime -- --nocapture
}

runtime_run_linux_container_runtime_lane() {
	container_engine="$1"
	repo_root="$2"
	toolchain_channel="$3"

	container_image="${DESTACK_RUNTIME_LINUX_CONTAINER_IMAGE:-rust:bookworm}"
	container_rustflags="${DESTACK_RUNTIME_LINUX_CONTAINER_RUSTFLAGS:--C force-frame-pointers=yes -Z threads=1}"

	"${container_engine}" run --rm \
		-v "${repo_root}:/work" \
		-v destack-runtime-linux-cargo-registry:/usr/local/cargo/registry \
		-v destack-runtime-linux-cargo-git:/usr/local/cargo/git \
		-v destack-runtime-linux-rustup:/usr/local/rustup \
		-w /tmp \
		"${container_image}" \
		bash -lc "
			set -euo pipefail
			export PATH=/usr/local/cargo/bin:\$PATH
			apt-get update >/dev/null
			apt-get install -y pkg-config python3 >/dev/null
			rustup toolchain install '${toolchain_channel}' --profile minimal --component clippy >/dev/null
			rustup default '${toolchain_channel}' >/dev/null
			source /work/dev/toolchain/lib/runtime-common.sh
			runtime_set_standard_environment
			export CARGO_BUILD_JOBS=1
			export CARGO_PROFILE_DEV_DEBUG=0
			export CARGO_PROFILE_TEST_DEBUG=0
			export RUSTFLAGS='${container_rustflags}'
			export CARGO_TARGET_DIR=/work/target/runtime-linux-container
			LC_ALL=C LANG=C LC_CTYPE=C CARGO_INCREMENTAL=0 cargo check --manifest-path /work/Cargo.toml -p destack_runtime
			LC_ALL=C LANG=C LC_CTYPE=C CARGO_INCREMENTAL=0 cargo clippy --manifest-path /work/Cargo.toml -p destack_runtime --all-targets --no-deps -- -D warnings
			LC_ALL=C LANG=C LC_CTYPE=C CARGO_INCREMENTAL=0 cargo test --manifest-path /work/Cargo.toml -p destack_runtime -- --nocapture
		"
}

runtime_auto_install_toolchains_enabled() {
	auto_install="${DESTACK_AUTO_INSTALL_TOOLCHAINS:-0}"
	[ "${auto_install}" = "1" ]
}

runtime_linux_install_package() {
	package_name="$1"

	if [ "$(runtime_host_kernel)" != "Linux" ]; then
		return 1
	fi

	if ! command -v apt-get >/dev/null 2>&1; then
		echo "cannot auto install ${package_name}: apt-get is not available"
		return 1
	fi

	if [ "$(id -u)" -eq 0 ]; then
		apt-get update
		apt-get install -y "${package_name}"
		return "$?"
	fi

	if ! command -v sudo >/dev/null 2>&1; then
		echo "cannot auto install ${package_name}: sudo is required on non-root linux hosts"
		return 1
	fi

	sudo apt-get update
	sudo apt-get install -y "${package_name}"
}

runtime_darwin_install_package() {
	package_name="$1"

	if [ "$(runtime_host_kernel)" != "Darwin" ]; then
		return 1
	fi

	if ! command -v brew >/dev/null 2>&1; then
		echo "cannot auto install ${package_name}: brew is not available"
		return 1
	fi

	brew install "${package_name}"
}

runtime_require_or_auto_install_command() {
	command_name="$1"
	package_name="$2"
	missing_message="$3"
	auto_install_hint="$4"

	if [ "${command_name}" = "java" ]; then
		if [ -n "$(runtime_java_path)" ]; then
			return 0
		fi
	else
		if runtime_require_command "${command_name}" "${missing_message}" >/dev/null 2>&1; then
			return 0
		fi
	fi

	if ! runtime_auto_install_toolchains_enabled; then
		echo "${missing_message}"
		if [ -n "${auto_install_hint}" ]; then
			echo "or run with auto install: DESTACK_AUTO_INSTALL_TOOLCHAINS=1 ${auto_install_hint}"
		fi
		return 1
	fi

	host_kernel="$(runtime_host_kernel)"

	if [ "${host_kernel}" = "Linux" ]; then
		echo "auto install requested: installing ${package_name}"
		if ! runtime_linux_install_package "${package_name}"; then
			return 1
		fi
	elif [ "${host_kernel}" = "Darwin" ]; then
		echo "auto install requested: installing ${package_name}"
		if ! runtime_darwin_install_package "${package_name}"; then
			return 1
		fi
	else
		echo "${missing_message}"
		echo "auto install is not supported on ${host_kernel} hosts for ${package_name}"
		return 1
	fi

	if [ "${command_name}" = "java" ]; then
		[ -n "$(runtime_java_path)" ]
	else
		return 0
	fi

	runtime_require_command "${command_name}" "${missing_message}" >/dev/null 2>&1
}

runtime_detect_timeout_command() {
	if command -v timeout >/dev/null 2>&1; then
		printf '%s\n' timeout
		return 0
	fi

	if command -v gtimeout >/dev/null 2>&1; then
		printf '%s\n' gtimeout
		return 0
	fi

	printf '%s\n' ""
}

runtime_run_with_optional_timeout() {
	timeout_seconds="$1"
	timeout_message="$2"
	shift 2

	if [ "${timeout_seconds}" = "0" ]; then
		"$@"
		return "$?"
	fi

	timeout_command="$(runtime_detect_timeout_command)"
	if [ -n "${timeout_command}" ]; then
		"${timeout_command}" --signal=TERM --kill-after=10 "${timeout_seconds}s" "$@"
		return "$?"
	fi

	timeout_marker="$(mktemp)"
	"$@" &
	command_pid="$!"

	(
		sleep "${timeout_seconds}"
		echo timeout >"${timeout_marker}"
		kill -TERM "${command_pid}" 2>/dev/null || true
		sleep 10
		kill -KILL "${command_pid}" 2>/dev/null || true
	) &
	watchdog_pid="$!"

	set +e
	wait "${command_pid}"
	command_status="$?"
	set -e

	kill "${watchdog_pid}" 2>/dev/null || true
	wait "${watchdog_pid}" 2>/dev/null || true

	if [ -s "${timeout_marker}" ]; then
		rm -f "${timeout_marker}"
		echo "${timeout_message} timed out after ${timeout_seconds}s"
		return 124
	fi

	rm -f "${timeout_marker}"
	return "${command_status}"
}

runtime_collect_cargo_test_executables() {
	cargo_output_file="$1"
	package_name="$2"

	runtime_require_command python3 "missing python3: install python3 for cargo json artifact parsing" || return 1

	python3 - "${cargo_output_file}" "${package_name}" <<'PY'
import json
import sys

output_file = sys.argv[1]
package_name = sys.argv[2]
package_prefix = f"{package_name} "
package_anchor = f"#{package_name}@"

executables = []
seen = set()

with open(output_file, "r", encoding="utf-8", errors="replace") as handle:
    for line in handle:
        line = line.strip()
        if not line:
            continue

        try:
            payload = json.loads(line)
        except Exception:
            continue

        if payload.get("reason") != "compiler-artifact":
            continue

        package_id = payload.get("package_id", "")
        if not isinstance(package_id, str):
            continue

        # match both legacy "crate version (...)" and modern "path+...#crate@version" package ids
        package_matches = package_id.startswith(package_prefix) or package_anchor in package_id
        if not package_matches:
            continue

        profile = payload.get("profile", {})
        if not isinstance(profile, dict) or not profile.get("test", False):
            continue

        executable = payload.get("executable")
        if not isinstance(executable, str) or not executable.endswith(".exe"):
            continue

        if executable in seen:
            continue

        seen.add(executable)
        executables.append(executable)

for executable in executables:
    print(executable)
PY
}
