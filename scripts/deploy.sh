#!/bin/bash

# check that the repo is clean
if [[ -n $(git status --porcelain) && "$1" != "--force" ]]; then
    echo "Repo is not clean. Aborting."
    exit 1
fi

# call build_push.sh
./scripts/build_push.sh

# deploy with pulumi
pulumi --cwd infra up -y --skip-preview