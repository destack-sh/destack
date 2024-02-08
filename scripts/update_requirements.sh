#!/bin/bash

# update all requirements files from their .in
python -m piptools compile requirements.in
python -m piptools compile requirements-runtime.in
python -m piptools compile requirements.in requirements-dev.in --output-file requirements-dev.txt
python -m piptools compile requirements-runtime.in requirements-runtime-dev.in --output-file requirements-runtime-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    ./venv/bin/python -m piptools sync requirements-dev.txt
    ./venv-runtime/bin/python -m piptools sync requirements-runtime-dev.txt
fi