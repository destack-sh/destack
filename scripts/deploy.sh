#!/bin/bash

# check that the repo is clean
if [[ -n $(git status --porcelain) && "$1" != "--force" ]]; then
    echo "Repo is not clean. Aborting."
    exit 1
fi

# build
./scripts/build.sh $1

# deploy with terraform
terraform apply -chdir=infra -auto-approve -var-file=infra/prod.tfvars
