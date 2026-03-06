#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
grammar_directory="$(cd "${script_directory}/.." && pwd)"
package_directory="${grammar_directory}/destack"
tree_sitter_cli="tree-sitter-cli@0.24.4"

ensure_dependencies() {
	if [[ -d "${package_directory}/node_modules/tree-sitter-cli" ]] &&
		[[ -d "${package_directory}/node_modules/tree-sitter-javascript" ]]; then
		return
	fi

	# install the grammar package dependencies
	(
		cd "${package_directory}"
		npm ci
	)
}

generate_dialect() {
	local dialect_directory="${package_directory}/$1"

	# regenerate the tracked parser sources for this dialect
	(
		cd "${dialect_directory}"
		bunx "${tree_sitter_cli}" generate
	)
}

test_typescript_tsx() {
	generate_dialect "typescript"
	generate_dialect "tsx"

	# run the shared TypeScript and TSX corpus
	(
		cd "${package_directory}"
		bunx "${tree_sitter_cli}" test --rebuild
	)
}

test_destack() {
	generate_dialect "destack"

	# run the destack specific corpus
	(
		cd "${package_directory}/destack"
		bunx "${tree_sitter_cli}" test --rebuild
	)
}

main() {
	local mode="${1:-}"

	if [[ -z "${mode}" ]]; then
		echo "usage: $0 <typescript-tsx|destack>" >&2
		exit 1
	fi

	ensure_dependencies

	case "${mode}" in
	"typescript-tsx")
		test_typescript_tsx
		;;
	"destack")
		test_destack
		;;
	*)
		echo "unknown mode: ${mode}" >&2
		exit 1
		;;
	esac
}

main "$@"
