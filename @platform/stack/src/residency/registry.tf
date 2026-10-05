resource "cloudflare_r2_bucket" "packages" {
  account_id    = var.account_id
  name          = "destack-${var.environment}-packages-${var.residency}"
  jurisdiction  = var.residency == "eu" ? "eu" : "default"
  storage_class = "Standard"
  lifecycle {
    prevent_destroy = true
  }
}
