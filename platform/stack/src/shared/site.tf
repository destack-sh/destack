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
