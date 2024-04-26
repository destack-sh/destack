#!/bin/bash

# update all requirements files from their .in
uv pip compile requirements.in
uv pip compile requirements-runtime.in
uv pip compile requirements.in requirements-dev.in --output-file requirements-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source ./venv-runtime/bin/activate && uv pip sync requirements-runtime.txt
    source ./venv/bin/activate && uv pip sync requirements-dev.txt
fi