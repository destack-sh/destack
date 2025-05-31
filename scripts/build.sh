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

# build bench-ts with :BenchWebEnv placeholders (to be substituted in deploy)
# (we need to 'set' them explicitly or they will be removed by vite during the build)
VITE_COMMIT="VITE_COMMIT" \
  VITE_ENVIRONMENT="VITE_ENVIRONMENT" \
  VITE_SUPERVISOR_URL="VITE_SUPERVISOR_URL" \
  VITE_IP_API_KEY="VITE_IP_API_KEY" \
  bun run --cwd bench-ts build
# posthog sourcemaps
posthog-cli --host https://eu.posthog.com sourcemap inject --directory bench-ts/dist/assets
posthog-cli --host https://eu.posthog.com sourcemap upload --directory bench-ts/dist/assets
rm bench-ts/dist/assets/*.map

# push Docker image with retries
push_with_retry() {
  local tag=$1
  local max_attempts=3
  local attempt=1

  while [ $attempt -le $max_attempts ]; do
    if docker push $tag; then
      return 0
    else
      if [ $attempt -lt $max_attempts ]; then
        sleep 5
      fi
      attempt=$((attempt + 1))
    fi
  done

  return 1
}

# build all images
# image names and their corresponding Dockerfiles
IMAGES=("bench-system" "bench-machine-runtime")
DOCKERFILES=("bench-infra/docker/Dockerfile.system" "bench-infra/docker/Dockerfile.machine-runtime")

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

  # push to GHCR with retry logic
  push_with_retry "ghcr.io/symbolx/$IMAGE:latest"
  push_with_retry "ghcr.io/symbolx/$IMAGE:$GIT_COMMIT"
  push_with_retry "ghcr.io/symbolx/$IMAGE:$VERSION"
done
