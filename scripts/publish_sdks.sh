#!/bin/bash
set -euo pipefail

check_env_var() {
    local var=$1
    if [[ -z "${!var:-}" ]]; then
        echo "Environment variable $var is required" >&2
        exit 1
    fi
}

for var in PYPI_TOKEN NPM_TOKEN CARGO_REGISTRY_TOKEN RUBYGEMS_API_KEY; do
    check_env_var "$var"
done

# Publish Python package
(
    cd destack-py-sdk
    python -m build
    twine upload --non-interactive -u __token__ -p "$PYPI_TOKEN" dist/*
    rm -rf build dist *.egg-info
)

# Publish JavaScript package
(
    cd destack-ts-sdk
    echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" > .npmrc
    npm publish --access public
    rm .npmrc
)

# Publish Rust crate
(
    cd destack-rs-sdk
    cargo package
    cargo publish --token "$CARGO_REGISTRY_TOKEN"
)

# Publish Ruby gem
(
    cd destack-rb-sdk
    gem build destack.gemspec
    mkdir -p ~/.gem
    printf "---\n:rubygems_api_key: %s\n" "$RUBYGEMS_API_KEY" > ~/.gem/credentials
    chmod 0600 ~/.gem/credentials
    gem push destack-*.gem
)

