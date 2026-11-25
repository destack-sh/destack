# keep the bucket the residency root created under its region name
moved {
  from = cloudflare_r2_bucket.residency
  to   = cloudflare_r2_bucket.region
}

# keep the region's data in one bucket in its jurisdiction
resource "cloudflare_r2_bucket" "region" {
  account_id    = var.account_id
  name          = "destack-${var.environment}-${var.region}"
  jurisdiction  = var.region
  storage_class = "Standard"

  lifecycle {
    prevent_destroy = true
  }
}
