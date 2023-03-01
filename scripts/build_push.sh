#!/bin/bash

# Abort if repo is not clean and not --force
if [[ -n $(git status --porcelain) && "$1" != "--force" ]]; then
  echo "Repo is not clean. Aborting."
  exit 1
fi

# Get current commit hash
GIT_COMMIT=$(git rev-parse --short HEAD)
# Get version from 'version' file
VERSION=$(cat version)

# Build the Docker API image and tag properly (with commit hash)
docker build . \
  -f Dockerfile \
  -t symbolx/bench-api:latest \
  -t symbolx/bench-api:GIT_COMMIT \
  -t ghcr.io/symbolx/bench-api:latest \
  -t ghcr.io/symbolx/bench-api:GIT_COMMIT \
  --build-arg GIT_COMMIT=GIT_COMMIT \
  --build-arg VERSION=VERSION

# push to GHCR
docker push ghcr.io/symbolx/bench-api:latest
docker push ghcr.io/symbolx/bench-api:GIT_COMMIT
docker push ghcr.io/symbolx/bench-api:VERSION