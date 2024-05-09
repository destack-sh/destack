#!/bin/bash

# update all requirements files from their .in
uv pip compile requirements.in --output-file requirements.txt
uv pip compile requirements-runtime.in --output-file requirements-runtime.txt
uv pip compile requirements.in requirements-dev.in --output-file requirements-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source ./venv-runtime/bin/activate && uv pip sync requirements-runtime.txt
    source ./venv/bin/activate && uv pip sync requirements-dev.txt
fi