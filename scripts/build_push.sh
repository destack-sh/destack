#!/bin/bash

# Abort if repo is not clean and not --force
if [[ -n $(git status --porcelain) && "$1" != "--force" ]]; then
  echo "Repo is not clean. Aborting."
  exit 1
fi

# Get current commit hash
COMMIT_HASH=$(git rev-parse --short HEAD)

# Build the Docker API image and tag properly (with commit hash)
docker build . \
  -f Dockerfile \
  -t symbolx/bench-api:latest \
  -t symbolx/bench-api:$COMMIT_HASH \
  -t ghcr.io/symbolx/bench-api:latest \
  -t ghcr.io/symbolx/bench-api:$COMMIT_HASH \
  --build-arg COMMIT_HASH=$COMMIT_HASH

# push to GHCR
docker push ghcr.io/symbolx/bench-api:latest
docker push ghcr.io/symbolx/bench-api:$COMMIT_HASH