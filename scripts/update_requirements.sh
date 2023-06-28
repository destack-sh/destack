#!/bin/bash

# update all requirements files from their .in
python -m piptools compile requirements.in
python -m piptools compile requirements-worker.in
python -m piptools compile requirements.in requirements-dev.in --output-file requirements-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    python -m piptools sync requirements-dev.txt
fi

# create/update worker venv
python -m venv venv-worker
base_site_packages="$(python -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])')"
derived_site_packages="$(./venv-worker/bin/python -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])')"
echo "$base_site_packages" > "$derived_site_packages"/_base_packages.pth