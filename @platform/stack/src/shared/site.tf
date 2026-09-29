resource "cloudflare_dns_record" "site" {
  zone_id = cloudflare_zone.domains["destack.sh"].id
  name    = "destack.sh"
  type    = "AAAA"
  content = "100::"
  proxied = true
  ttl     = 1
}

resource "cloudflare_workers_route" "site" {
  pattern = "destack.sh/*"
  script  = "destack-site"
  zone_id = cloudflare_zone.domains["destack.sh"].id
}

resource "cloudflare_dns_record" "company" {
  zone_id = cloudflare_zone.domains["symbol.industries"].id
  name    = "symbol.industries"
  type    = "AAAA"
  content = "100::"
  proxied = true
  ttl     = 1
}

import {
  to = cloudflare_dns_record.company
  id = "643d8f0f0ea084d0c31c40098d7faec6/a4e33c4686147d6a9e58c4dcda7267e8"
}

resource "cloudflare_workers_route" "company" {
  pattern = "symbol.industries/*"
  script  = "symbol-company"
  zone_id = cloudflare_zone.domains["symbol.industries"].id
}
