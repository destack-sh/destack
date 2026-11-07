# forget the legacy bucket without deleting its objects
removed {
  from = cloudflare_r2_bucket.releases

  lifecycle {
    destroy = false
  }
}

# keep the feed's releases, named for its stable path
resource "cloudflare_r2_bucket" "release" {
  account_id    = local.account_id
  name          = "destack-releases-stable"
  storage_class = "Standard"

  lifecycle {
    prevent_destroy = true
  }
}

# serve every download from the bucket at its domain, each object with the type and cache policy its upload stored
resource "cloudflare_r2_custom_domain" "download" {
  account_id  = local.account_id
  bucket_name = cloudflare_r2_bucket.release.name
  domain      = "download.destack.sh"
  zone_id     = cloudflare_zone.domains["destack.sh"].id
  enabled     = true
  min_tls     = "1.2"
}
