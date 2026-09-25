#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
grammar_directory="$(cd "${script_directory}/.." && pwd)"
repository_directory="$(cd "${grammar_directory}/../.." && pwd)"
tree_sitter="${repository_directory}/node_modules/.bin/tree-sitter"
build_directory="$(mktemp -d)"
config_file="${build_directory}/config.json"

case "$(uname -s)" in
Darwin) library_extension="dylib" ;;
Linux) library_extension="so" ;;
*) library_extension="dll" ;;
esac

cleanup() {
    rm -rf "${build_directory}"
}

trap cleanup EXIT

printf '{"parser-directories":["%s"]}\n' "${grammar_directory}" > "${config_file}"

generate_grammar() {
    local grammar="$1"
    local directory="${grammar_directory}/${grammar}"

    # regenerate the tracked parser at the current language ABI
    (
        cd "${directory}"
        "${tree_sitter}" generate --abi 15
    )

}

test_corpus() {
    local grammar="$1"
    local directory="${grammar_directory}/${grammar}"
    local library="${build_directory}/${grammar}.${library_extension}"
    local language="${grammar}"

    # use the exported language name when it differs from the directory
    if [[ "${grammar}" == "bytecode" ]]; then
        language="tspp_bytecode"
    fi

    # compile the generated parser once for corpus and query tests
    "${tree_sitter}" build --output "${library}" "${directory}"

    # exercise the complete corpus through the explicit parser library
    (
        cd "${directory}"
        "${tree_sitter}" test --lib-path "${library}" --lang-name "${language}"
    )
}

test_tspp_queries() {
    local directory="${grammar_directory}/tspp"
    local example="${directory}/test/highlight/tspp.tspp"
    local library="${build_directory}/tspp.${library_extension}"

    # compile and execute every configured query against one TS++ source file
    while IFS= read -r query; do
        "${tree_sitter}" query \
            --config-path "${config_file}" \
            --lib-path "${library}" \
            --lang-name tspp \
            --quiet \
            "${directory}/${query}" \
            "${example}" \
            >/dev/null
    done < <(node "${script_directory}/query-paths.mjs" "${directory}/tree-sitter.json")

    # assert representative captures through the highlight test format
    "${tree_sitter}" highlight \
        --config-path "${config_file}" \
        --grammar-path "${directory}" \
        --quiet \
        "${example}" \
        >/dev/null
}

main() {
    local grammar="${1:-}"

    # reject unknown grammar names before writing generated files
    case "${grammar}" in
    tspp | mir | bytecode) ;;
    *)
        echo "usage: $0 <tspp|mir|bytecode>" >&2
        exit 1
        ;;
    esac

    generate_grammar "${grammar}"
    test_corpus "${grammar}"

    # tspp is the only grammar with editor queries beyond highlighting
    if [[ "${grammar}" == "tspp" ]]; then
        test_tspp_queries
    fi
}

main "$@"
