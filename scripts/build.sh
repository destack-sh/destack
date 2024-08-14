#!/bin/bash
set -e

# abort if repo is not clean and not --force
if [[ -n $(git status --porcelain) && "$1" != "--force" ]]; then
  echo "Repo is not clean. Aborting."
  exit 1
fi

# get current commit hash
GIT_COMMIT=$(git rev-parse --short HEAD)
# get version from 'version' file
VERSION=$(cat version)

bun run --cwd bench-web build

# build docker images
# image names
IMAGES=("bench-system" "bench-runtime")
for IMAGE in ${IMAGES[@]}; do
  # Build the Docker image and tag it properly (with commit hash)
  docker build . \
    --target $IMAGE \
    --platform linux/amd64 \
    -f bench-infra/docker/Dockerfile.bench \
    -t symbolx/$IMAGE:latest \
    -t symbolx/$IMAGE:$GIT_COMMIT \
    -t symbolx/$IMAGE:$VERSION \
    -t ghcr.io/symbolx/$IMAGE:latest \
    -t ghcr.io/symbolx/$IMAGE:$GIT_COMMIT \
    -t ghcr.io/symbolx/$IMAGE:$VERSION \
    --build-arg GIT_COMMIT=$GIT_COMMIT \
    --build-arg VERSION=$VERSION

  # push to GHCR
  docker push ghcr.io/symbolx/$IMAGE:latest
  docker push ghcr.io/symbolx/$IMAGE:$GIT_COMMIT
  docker push ghcr.io/symbolx/$IMAGE:$VERSION
done
