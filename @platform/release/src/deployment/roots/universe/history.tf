# keep the account service's history segments
resource "cloudflare_r2_bucket" "history" {
  account_id    = var.account_id
  name          = "destack-${var.environment}-history"
  storage_class = "Standard"
  lifecycle {
    prevent_destroy = true
  }
}
