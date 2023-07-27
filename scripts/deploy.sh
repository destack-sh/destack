#!/bin/bash

# fetch infra .vector config if not present
if [ ! -f infra/.vector.prod.yaml ]; then
    echo "Fetching .vector.values"
    curl https://logs.betterstack.com/vector-helm/8gmSKZjoNvxPvFsuAKRDxFKP > infra/.vector.prod.yaml
fi

# check that the repo is clean
if [[ -n $(git status --porcelain) && "$1" != "--force" ]]; then
    echo "Repo is not clean. Aborting."
    exit 1
fi

# call build_push.sh
./scripts/build_push.sh $1

# deploy with pulumi
pulumi --cwd infra up -y --skip-preview
