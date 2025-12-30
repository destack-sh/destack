#!/usr/bin/env bash

fail() {
    echo "$1" >&2
    exit 1
}

fetch_url() {
    local url="$1"
    local dest="$2"
    local tmp_file

    tmp_file="$(mktemp)"

    if ! curl -fsSL "$url" > "$tmp_file"; then
        rm -f "$tmp_file"
        fail "missing resource $url"
    fi

    if grep -q "^Not found:" "$tmp_file"; then
        rm -f "$tmp_file"
        fail "missing resource $url"
    fi

    if [[ ! -s "$tmp_file" ]]; then
        rm -f "$tmp_file"
        fail "empty response from $url"
    fi

    mkdir -p "$(dirname "$dest")"
    mv "$tmp_file" "$dest"
}

fetch_ts_lib() {
    local source="$1"
    local dest="$2"

    fetch_url "$BASE_URL/$source" "$dest"
}
