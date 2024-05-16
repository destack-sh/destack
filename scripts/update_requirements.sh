#!/bin/bash

# update all requirements files from their .in
uv pip compile requirements/system.in --output-file requirements/system.txt
uv pip compile requirements/system.in requirements/dev.in --output-file requirements/system-dev.txt
uv pip compile requirements/runtime.in --output-file requirements/runtime.txt
uv pip compile requirements/runtime.in requirements/dev.in --output-file requirements/runtime-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source ./venv-runtime/bin/activate && uv pip sync requirements/runtime-dev.txt
    source ./venv/bin/activate && uv pip sync requirements/system-dev.txt
fi
