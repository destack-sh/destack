#!/bin/bash

# build
./scripts/build.sh $1

# deploy with terraform
terraform apply -chdir=infra -auto-approve -var-file=infra/prod.tfvars

# invalidate cloudfront cache
aws cloudfront create-invalidation --distribution-id $(terraform output -json | jq -r '.bench_web_distribution_id.value') --paths "/*"
