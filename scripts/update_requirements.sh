#!/bin/bash

# update all requirements files from their .in
uv pip compile bench-py/bench/requirements/system.in --output-file bench-py/bench/requirements/system.txt
uv pip compile bench-py/bench/requirements/system.in bench-py/bench/requirements/dev.in --output-file bench-py/bench/requirements/system-dev.txt
uv pip compile bench-py/bench/requirements/runtime.in --output-file bench-py/bench/requirements/runtime.txt
uv pip compile bench-py/bench/requirements/runtime.in bench-py/bench/requirements/dev.in --output-file bench-py/bench/requirements/runtime-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source bench-py/venv/bin/activate && uv pip sync bench-py/bench/requirements/system-dev.txt
    source bench-py/venv-runtime/bin/activate && uv pip sync bench-py/bench/requirements/runtime-dev.txt
fi
