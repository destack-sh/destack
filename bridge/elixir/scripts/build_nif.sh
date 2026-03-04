#!/usr/bin/env bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_directory="$(cd "${script_directory}/.." && pwd)"

mkdir -p "${project_directory}/priv"

erlang_root="$(erl -noshell -eval 'io:format("~s", [code:root_dir()]).' -s init stop)"
include_directory="${erlang_root}/usr/include"

output_file="${project_directory}/priv/destack_nif.so"
source_file="${project_directory}/c_src/destack_nif.c"

if [[ "$(uname -s)" == "Darwin" ]]; then
	cc -O2 -fPIC -bundle -undefined dynamic_lookup -I"${include_directory}" -o "${output_file}" "${source_file}"
else
	cc -O2 -fPIC -shared -I"${include_directory}" -o "${output_file}" "${source_file}" -ldl
fi
