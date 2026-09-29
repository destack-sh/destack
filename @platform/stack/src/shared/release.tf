resource "cloudflare_r2_bucket" "releases" {
  account_id    = local.account_id
  name          = "destack-releases"
  storage_class = "Standard"

  lifecycle {
    prevent_destroy = true
  }
}

# separate release credentials at the bucket permission level
resource "cloudflare_r2_bucket" "release" {
  for_each = toset(["stable", "nightly"])

  account_id    = local.account_id
  name          = "destack-releases-${each.key}"
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

# route each signed release feed to its bucket through the publication worker
resource "cloudflare_workers_route" "release" {
  for_each = toset(["stable", "nightly"])

  zone_id = cloudflare_zone.domains["destack.sh"].id
  pattern = "download.destack.sh/${each.key}/*"
  script  = "destack-release-publication"
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
