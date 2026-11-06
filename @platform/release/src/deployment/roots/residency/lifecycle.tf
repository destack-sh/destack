# abort the multipart uploads a crashed pack upload leaves, a day after they start
resource "cloudflare_r2_bucket_lifecycle" "files" {
  account_id   = var.account_id
  bucket_name  = cloudflare_r2_bucket.files.name
  jurisdiction = cloudflare_r2_bucket.files.jurisdiction
  rules = [{
    id         = "abort-incomplete-uploads"
    enabled    = true
    conditions = { prefix = "" }
    abort_multipart_uploads_transition = {
      condition = { type = "Age", max_age = 86400 }
    }
  }]
}

# abort the incomplete uploads of the registry's bucket the same way
resource "cloudflare_r2_bucket_lifecycle" "packages" {
  account_id   = var.account_id
  bucket_name  = cloudflare_r2_bucket.packages.name
  jurisdiction = cloudflare_r2_bucket.packages.jurisdiction
  rules = [{
    id         = "abort-incomplete-uploads"
    enabled    = true
    conditions = { prefix = "" }
    abort_multipart_uploads_transition = {
      condition = { type = "Age", max_age = 86400 }
    }
  }]
}
