# keep the residency's data in one bucket in its jurisdiction
resource "cloudflare_r2_bucket" "residency" {
  account_id    = var.account_id
  name          = "destack-${var.environment}-${var.residency}"
  jurisdiction  = var.residency
  storage_class = "Standard"

  lifecycle {
    prevent_destroy = true
  }
}
