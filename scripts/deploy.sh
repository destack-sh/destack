#!/bin/bash
set -e

# deploy with terraform
terraform -chdir=infra apply -auto-approve -var-file=prod.tfvars

# invalidate cloudfront cache
aws cloudfront create-invalidation --distribution-id $(terraform output -json | jq -r '.bench_web_distribution_id.value') --paths "/*" || true
