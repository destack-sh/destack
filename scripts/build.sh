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

# build computer images
COMPUTER_IMAGES=("bench-computer-ubuntu")
for IMAGE in ${COMPUTER_IMAGES[@]}; do
  # Build the Docker image and tag it
  docker build . \
    -f bench-infra/docker/Dockerfile.computer-ubuntu \
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

# build bench-web with :BenchWebEnv placeholders (to be substituted in deploy)
# (we need to 'set' them explicitly or they will be removed by vite during the build)
VITE_COMMIT="VITE_COMMIT" VITE_ENV="VITE_ENV" VITE_SUPERVISOR_URL="VITE_SUPERVISOR_URL" VITE_IP_API_KEY="VITE_IP_API_KEY" bun run --cwd bench-web build

# build bench images
# image names
IMAGES=("bench-system" "bench-runtime")
for IMAGE in ${IMAGES[@]}; do
  # Build the Docker image and tag it
  docker build . \
    --target $IMAGE \
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
