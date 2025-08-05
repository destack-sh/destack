#!/bin/bash

#
# Python
# update all requirements files from their .in
#

# destack
uv pip compile \
    destack/requirements/dev.in \
    --output-file destack/requirements/requirements-dev.txt
source destack/venv/bin/activate && \
    uv pip sync destack/requirements/requirements-dev.txt && \
    uv pip install -e destack

# destack-py
uv pip compile \
    destack/requirements/dev.in \
    destack-py/requirements/dev.in \
    --output-file destack-py/requirements/requirements-dev.txt
source destack-py/venv/bin/activate && \
    uv pip sync destack-py/requirements/requirements-dev.txt

# destack-py-server
uv pip compile \
    destack-py-server/requirements/requirements.in \
    --output-file destack-py-server/requirements/requirements.txt
uv pip compile \
    destack/requirements/dev.in \
    destack-py/requirements/dev.in \
    destack-py-server/requirements/requirements.in \
    destack-py-server/requirements/dev.in \
    --output-file destack-py-server/requirements/requirements-dev.txt
source destack-py-server/venv/bin/activate && \
    uv pip sync destack-py-server/requirements/requirements-dev.txt &&

#
# Javascript
# 

bun i