# keep the test files that passed over each package closure, which every CI run and branch shares
resource "cloudflare_r2_bucket" "ci" {
  account_id    = local.account_id
  name          = "destack-ci"
  storage_class = "Standard"
}

# drop records nothing has rewritten in a month, as their closures are long gone
resource "cloudflare_r2_bucket_lifecycle" "ci" {
  account_id  = local.account_id
  bucket_name = cloudflare_r2_bucket.ci.name
  rules = [{
    id         = "expire-records"
    enabled    = true
    conditions = { prefix = "" }
    delete_objects_transition = {
      condition = { type = "Age", max_age = 2592000 }
    }
  }]
}

# find the permission to read and write the bucket's objects
data "cloudflare_account_api_token_permission_groups_list" "r2_write" {
  account_id = local.account_id
  name       = "Workers R2 Storage Bucket Item Write"
}

# let CI read and write the bucket's objects, and nothing else
resource "cloudflare_account_token" "ci" {
  account_id = local.account_id
  name       = "destack-ci"
  policies = [{
    effect            = "allow"
    permission_groups = [{ id = data.cloudflare_account_api_token_permission_groups_list.r2_write.result[0].id }]
    resources = jsonencode({
      "com.cloudflare.edge.r2.bucket.${local.account_id}_default_${cloudflare_r2_bucket.ci.name}" = "*"
    })
  }]
}

# the token CI signs its record requests with, kept as the repository secret DESTACK_TEST_STORE_TOKEN
output "ci_token" {
  value     = cloudflare_account_token.ci.value
  sensitive = true
}

# the token's identifier, the S3 access key R2 derives from it
output "ci_token_id" {
  value = cloudflare_account_token.ci.id
}
