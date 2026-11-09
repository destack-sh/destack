# answer every installation's and machine's one-label name, which the platform residency's machine routes
resource "cloudflare_dns_record" "edge" {
  for_each = toset(["destack.computer", "destack.space"])
  zone_id  = cloudflare_zone.domains[each.key].id
  name     = "*.${each.key}"
  type     = "AAAA"
  content  = "100::"
  proxied  = true
  ttl      = 1
}
