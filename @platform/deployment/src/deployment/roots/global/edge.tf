# answer every name the platform residency's machine routes: installations, machines and the universe
locals {
  edge = {
    "*.destack.computer" = "destack.computer"
    "*.destack.space"    = "destack.space"
    "destack.app"        = "destack.app"
  }
}

resource "cloudflare_dns_record" "edge" {
  for_each = local.edge
  zone_id  = cloudflare_zone.domains[each.value].id
  name     = each.key
  type     = "AAAA"
  content  = "100::"
  proxied  = true
  ttl      = 1
}

moved {
  from = cloudflare_dns_record.edge["destack.computer"]
  to   = cloudflare_dns_record.edge["*.destack.computer"]
}

moved {
  from = cloudflare_dns_record.edge["destack.space"]
  to   = cloudflare_dns_record.edge["*.destack.space"]
}
