#!/bin/bash

#
# Python
# update all requirements files from their .in
#

# destack
uv pip compile \
    destack-ds/destack/requirements/dev.in \
    --output-file destack-ds/destack/requirements/requirements-dev.txt
source destack-ds/venv/bin/activate && \
    uv pip sync destack-ds/destack/requirements/requirements-dev.txt

# destack-py
uv pip compile \
    destack-py/destack/requirements/dev.in \
    --output-file destack-py/destack/requirements/requirements-dev.txt
source destack-py/venv/bin/activate && \
    uv pip sync destack-py/destack/requirements/requirements-dev.txt

#
# Javascript
# 

bun i

#
# Rust
#

cargo update