#!/bin/bash

# update all requirements files from their .in
uv pip compile destack-py/destack/requirements/system.in --output-file destack-py/destack/requirements/system.txt
uv pip compile destack-py/destack/requirements/system.in destack-py/destack/requirements/dev.in --output-file destack-py/destack/requirements/system-dev.txt
uv pip compile destack-py/destack/requirements/runtime.in --output-file destack-py/destack/requirements/runtime.txt
uv pip compile destack-py/destack/requirements/runtime.in destack-py/destack/requirements/dev.in --output-file destack-py/destack/requirements/runtime-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source destack-py/venv/bin/activate && uv pip sync destack-py/destack/requirements/system-dev.txt
    source destack-py/venv-runtime/bin/activate && uv pip sync destack-py/destack/requirements/runtime-dev.txt
fi
