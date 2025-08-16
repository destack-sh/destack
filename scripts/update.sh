#!/bin/bash

#
# Python
# update all requirements files from their .in
#

# destack
uv pip compile \
    destack-language/destack/requirements/dev.in \
    --output-file destack-language/destack/requirements/requirements-dev.txt
source destack-language/destack/venv/bin/activate && \
    uv pip sync destack-language/destack/requirements/requirements-dev.txt && \
    uv pip install -e destack-language/destack

# destack-py
uv pip compile \
    destack-py/destack/requirements/dev.in \
    --output-file destack-py/destack/requirements/requirements-dev.txt
source destack-py/destack/venv/bin/activate && \
    uv pip sync destack-py/destack/requirements/requirements-dev.txt && \
    uv pip install -e destack-py/destack

#
# Javascript
# 

bun i

#
# Rust
#

cargo update