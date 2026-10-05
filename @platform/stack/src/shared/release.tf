# forget the legacy bucket without deleting its objects
removed {
  from = cloudflare_r2_bucket.releases

  lifecycle {
    destroy = false
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

resource "cloudflare_dns_record" "download" {
  zone_id = cloudflare_zone.domains["destack.sh"].id
  name    = "download.destack.sh"
  type    = "AAAA"
  content = "100::"
  proxied = true
  ttl     = 1
}

# serve every download through the publication worker
resource "cloudflare_workers_route" "release" {
  zone_id = cloudflare_zone.domains["destack.sh"].id
  pattern = "download.destack.sh/*"
  script  = "destack-release-publication"
}
