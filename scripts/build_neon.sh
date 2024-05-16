#!/bin/bash

# build custom Neon dockerfile
docker build . -f Dockerfile.neon -t symbolx/neon-local:latest -t ghcr.io/symbolx/neon-local:latest

# push to GHCR
docker push ghcr.io/symbolx/neon-local:latest
