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

# Image names
IMAGES=("bench-system" "bench-runtime")

for IMAGE in ${IMAGES[@]}; do
  # Build the Docker image and tag properly (with commit hash)
  docker build . \
    --platform linux/amd64 \
    --target $IMAGE \
    -f Dockerfile \
    -t symbolx/$IMAGE:latest \
    -t symbolx/$IMAGE:$GIT_COMMIT \
    -t ghcr.io/symbolx/$IMAGE:latest \
    -t ghcr.io/symbolx/$IMAGE:$GIT_COMMIT \
    -t ghcr.io/symbolx/$IMAGE:$VERSION \
    --build-arg GIT_COMMIT=$GIT_COMMIT \
    --build-arg VERSION=$VERSION

  # Push to GHCR
  docker push ghcr.io/symbolx/$IMAGE:latest
  docker push ghcr.io/symbolx/$IMAGE:$GIT_COMMIT
  docker push ghcr.io/symbolx/$IMAGE:$VERSION
done
