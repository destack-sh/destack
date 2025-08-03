#!/bin/bash

# update all requirements files from their .in

uv pip compile \
    destack/requirements/dev.in \
    --output-file destack/requirements/requirements-dev.txt

uv pip compile \
    destack/requirements/dev.in \
    destack-py/requirements/dev.in \
    --output-file destack-py/requirements/requirements-dev.txt

uv pip compile \
    destack-py-server/requirements/requirements.in \
    --output-file destack-py-server/requirements/requirements.txt

uv pip compile \
    destack/requirements/dev.in \
    destack-py/requirements/dev.in \
    destack-py-server/requirements/requirements.in \
    destack-py-server/requirements/dev.in \
    --output-file destack-py-server/requirements/requirements-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    source destack/venv/bin/activate && \
        uv pip sync destack/destack/requirements/requirements-dev.txt
    source destack-py/venv/bin/activate && \
        uv pip sync destack-py/destack/requirements/requirements-dev.txt
    source destack-py-server/venv/bin/activate && \
        uv pip sync destack-py-server/destack_server/requirements/requirements-dev.txt
fi
