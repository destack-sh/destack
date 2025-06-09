#!/bin/bash
set -e

# deploy with terraform
terraform -chdir=destack-infra apply -auto-approve -var-file=prod.tfvars

# invalidate cloudfront cache
aws cloudfront create-invalidation --distribution-id $(terraform -chdir=destack-infra output -json | jq -r '.destack_web_distribution_id.value') --paths "/*" >/dev/null || true
