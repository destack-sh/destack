#!/usr/bin/env bash
set -euo pipefail

runtime_args=()
refresh_stubs="true"

while [ "$#" -gt 0 ]; do
	case "$1" in
	--domains)
		if [ "$#" -lt 2 ]; then
			echo "missing value for --domains"
			exit 1
		fi
		runtime_args+=("--domains" "$2")
		shift
		;;
	--domains=* | --domain=*)
		runtime_args+=("$1")
		;;
	--domain)
		if [ "$#" -lt 2 ]; then
			echo "missing value for --domain"
			exit 1
		fi
		runtime_args+=("--domain" "$2")
		shift
		;;
	--refresh-stubs)
		refresh_stubs="true"
		;;
	--no-refresh-stubs)
		refresh_stubs="false"
		;;
	*)
		echo "unsupported generate-bindings option: $1"
		echo "supported: --domain, --domains, --refresh-stubs, --no-refresh-stubs"
		exit 1
		;;
	esac
	shift
done

if [ "$refresh_stubs" = "true" ]; then
	runtime_args+=("--refresh-stubs")
fi

if [ "${#runtime_args[@]}" -gt 0 ]; then
	cargo run -p destack_runtime --features generate_bindings --bin generate-bindings --release -- "${runtime_args[@]}"
else
	cargo run -p destack_runtime --features generate_bindings --bin generate-bindings --release
fi

cargo fmt -p destack_runtime
echo "generated runtime binding catalog"
