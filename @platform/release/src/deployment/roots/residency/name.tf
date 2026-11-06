# answer the cloud machine at its one-label name and its buckets below its S3 name
locals {
  host = "${var.machine}--platform.destack.computer"
  s3   = "s3--${var.machine}--platform.destack.computer"
}

data "cloudflare_zone" "computer" {
  filter = {
    name = "destack.computer"
  }
}

resource "cloudflare_dns_record" "name" {
  for_each = toset([local.host, local.s3, "*.${local.s3}"])
  zone_id  = data.cloudflare_zone.computer.zone_id
  name     = each.key
  type     = "AAAA"
  content  = "100::"
  proxied  = true
  ttl      = 1
}

# cover the buckets' names, one label below the S3 name, which the zone's universal certificate leaves out
resource "cloudflare_certificate_pack" "s3" {
  zone_id               = data.cloudflare_zone.computer.zone_id
  type                  = "advanced"
  hosts                 = ["destack.computer", local.s3, "*.${local.s3}"]
  validation_method     = "txt"
  validity_days         = 90
  certificate_authority = "google"
  cloudflare_branding   = false
}
