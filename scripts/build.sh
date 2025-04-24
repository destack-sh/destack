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

# build bench-web with :BenchWebEnv placeholders (to be substituted in deploy)
# (we need to 'set' them explicitly or they will be removed by vite during the build)
VITE_COMMIT="VITE_COMMIT" \
VITE_ENV="VITE_ENV" \
VITE_SUPERVISOR_URL="VITE_SUPERVISOR_URL" \
VITE_IP_API_KEY="VITE_IP_API_KEY" \
bun run --cwd bench-web build

# build all images
# image names and their corresponding Dockerfiles
IMAGES=("bench-computer-ubuntu-desktop" "bench-computer-ubuntu-terminal" "bench-system")
DOCKERFILES=("bench-infra/docker/Dockerfile.computer-ubuntu-desktop" "bench-infra/docker/Dockerfile.computer-ubuntu-terminal" "bench-infra/docker/Dockerfile.system")

for i in "${!IMAGES[@]}"; do
  IMAGE="${IMAGES[$i]}"
  DOCKERFILE="${DOCKERFILES[$i]}"
  docker build . \
    --platform linux/arm64 \
    -f "$DOCKERFILE" \
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
