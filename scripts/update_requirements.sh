
# update all requirements files from their .in
python -m piptools compile requirements.in
python -m piptools compile requirements.in requirements-compute.in --output-file requirements-compute.txt
python -m piptools compile requirements.in requirements-compute.in requirements-dev.in --output-file requirements-dev.txt

# optionally also sync packages with --sync
if [ "$1" == "--sync" ]; then
    python -m piptools sync requirements-dev.txt
fi