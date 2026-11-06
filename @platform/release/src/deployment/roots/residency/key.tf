# find R2's object read and write permission groups by their names, as the account lists them
data "cloudflare_account_api_token_permission_groups_list" "account" {
  account_id = var.account_id
}

locals {
  # the jurisdiction the residency's buckets live in, as R2 names it in resource keys
  jurisdiction = var.residency == "eu" ? "eu" : "default"

  # the permission groups reading and writing objects
  object_groups = [
    for group in data.cloudflare_account_api_token_permission_groups_list.account.result :
    { id = group.id }
    if contains(["Workers R2 Storage Bucket Item Read", "Workers R2 Storage Bucket Item Write"], group.name)
  ]
}

# give the cloud machine an R2 key limited to objects in the residency's two buckets
resource "cloudflare_account_token" "machine" {
  account_id = var.account_id
  name       = "destack-${var.environment}-machine-${var.residency}"
  policies = [{
    effect            = "allow"
    permission_groups = local.object_groups
    resources = jsonencode({
      for bucket in [cloudflare_r2_bucket.files.name, cloudflare_r2_bucket.packages.name] :
      "com.cloudflare.edge.r2.bucket.${var.account_id}_${local.jurisdiction}_${bucket}" => "*"
    })
  }]

  lifecycle {
    # refuse a token without both object groups, as a renamed group would leave it
    precondition {
      condition     = length(local.object_groups) == 2
      error_message = "R2's object read and write permission groups are missing from the account."
    }
  }
}
