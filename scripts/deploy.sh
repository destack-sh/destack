#!/bin/bash
set -e

# deploy with terraform
terraform -chdir=bench-infra apply -auto-approve -var-file=prod.tfvars

# invalidate cloudfront cache
aws cloudfront create-invalidation --distribution-id $(terraform -chdir=bench-infra output -json | jq -r '.bench_web_distribution_id.value') --paths "/*" || true
