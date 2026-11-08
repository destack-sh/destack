# answer the cloud machine at its one-label name
locals {
  host = "${var.machine}--platform.destack.computer"
}

data "cloudflare_zone" "computer" {
  filter = {
    name = "destack.computer"
  }
}

resource "cloudflare_dns_record" "name" {
  for_each = toset([local.host])
  zone_id  = data.cloudflare_zone.computer.zone_id
  name     = each.key
  type     = "AAAA"
  content  = "100::"
  proxied  = true
  ttl      = 1
}
