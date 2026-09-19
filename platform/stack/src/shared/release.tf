resource "cloudflare_r2_bucket" "releases" {
  account_id    = local.account_id
  name          = "destack-releases"
  storage_class = "Standard"

  lifecycle {
    prevent_destroy = true
  }
}

resource "cloudflare_r2_custom_domain" "releases" {
  account_id  = local.account_id
  bucket_name = cloudflare_r2_bucket.releases.name
  domain      = "download.destack.sh"
  zone_id     = cloudflare_zone.domains["destack.sh"].id
  enabled     = true
  min_tls     = "1.2"
}

resource "cloudflare_r2_bucket_cors" "releases" {
  account_id  = local.account_id
  bucket_name = cloudflare_r2_bucket.releases.name
  rules = [{
    allowed = {
      methods = ["GET", "HEAD"]
      origins = ["*"]
    }
  }]
}
