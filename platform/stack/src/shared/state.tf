terraform {
  backend "s3" {
    bucket                      = "destack-infrastructure"
    key                         = "platform.tfstate"
    region                      = "auto"
    use_lockfile                = true
    skip_credentials_validation = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_metadata_api_check     = true
    skip_s3_checksum            = true
    endpoints = {
      s3 = "https://27c0d00fb3a27a4ccbf46a3cceab9301.r2.cloudflarestorage.com"
    }
  }
}

resource "cloudflare_r2_bucket" "infrastructure" {
  account_id    = local.account_id
  name          = "destack-infrastructure"
  storage_class = "Standard"

  lifecycle {
    prevent_destroy = true
  }
}
