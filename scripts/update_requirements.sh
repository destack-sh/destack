#!/bin/bash

# update all requirements files from their .in
python -m piptools compile requirements.in
python -m piptools compile requirements-worker.in
python -m piptools compile requirements.in requirements-dev.in --output-file requirements-dev.txt
python -m piptools compile requirements-worker.in requirements-worker-dev.in --output-file requirements-worker-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    ./venv/bin/python -m piptools sync requirements-dev.txt
    ./venv-worker/bin/python -m piptools sync requirements-worker-dev.txt
fi