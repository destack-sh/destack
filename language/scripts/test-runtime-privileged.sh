#!/usr/bin/env bash
set -euo pipefail

if [ "$(id -u)" != "0" ]; then
	echo "test-runtime-privileged requires elevated privileges (run as root/admin)"
	exit 1
fi

LC_ALL=C LANG=C LC_CTYPE=C DESTACK_TEST_PRIVILEGED=1 just _run-rust-test -p destack_runtime platform::process::tests:: -- --nocapture
LC_ALL=C LANG=C LC_CTYPE=C DESTACK_TEST_PRIVILEGED=1 just _run-rust-test -p destack_runtime platform::fs::tests:: -- --nocapture
LC_ALL=C LANG=C LC_CTYPE=C DESTACK_TEST_PRIVILEGED=1 just _run-rust-test -p destack_runtime platform::net::tests:: -- --nocapture
