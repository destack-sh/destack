#!/bin/bash

# update all requirements files from their .in
uv pip compile bench/requirements/system.in --output-file bench/requirements/system.txt
uv pip compile bench/requirements/system.in bench/requirements/dev.in --output-file bench/requirements/system-dev.txt
uv pip compile bench/requirements/runtime.in --output-file bench/requirements/runtime.txt
uv pip compile bench/requirements/runtime.in bench/requirements/dev.in --output-file bench/requirements/runtime-dev.txt
uv pip compile bench/requirements/computer.in --output-file bench/requirements/computer.txt
uv pip compile bench/requirements/computer.in bench/requirements/dev.in --output-file bench/requirements/computer-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source ./venv/bin/activate && uv pip sync bench/requirements/system-dev.txt
    source ./venv-runtime/bin/activate && uv pip sync bench/requirements/runtime-dev.txt
    source ./venv-computer/bin/activate && uv pip sync bench/requirements/computer-dev.txt
fi
