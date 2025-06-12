#!/bin/bash
set -euo pipefail

# Python package
(
	cd destack-py-sdk
	python -m build
	twine upload --non-interactive -u __token__ -p "$PYPI_TOKEN" dist/*
	rm -rf build dist *.egg-info
)

# JavaScript package
(
	cd destack-ts-sdk
	echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" >.npmrc
	npm publish --access public
	rm .npmrc
)

# Rust crate
(
	cd destack-rs-sdk
	cargo package
	cargo publish --token "$CARGO_TOKEN"
)