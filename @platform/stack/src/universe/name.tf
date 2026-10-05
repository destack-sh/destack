# answer each process's zone routes through a proxied name of its zone
locals {
  names = toset(flatten([for process in values(var.processes) : process.names]))
}

data "cloudflare_zone" "name" {
  for_each = local.names
  filter = {
    name = join(".", slice(split(".", each.key), length(split(".", each.key)) - 2, length(split(".", each.key))))
  }
}

resource "cloudflare_dns_record" "name" {
  for_each = local.names
  zone_id  = data.cloudflare_zone.name[each.key].zone_id
  name     = each.key
  type     = "AAAA"
  content  = "100::"
  proxied  = true
  ttl      = 1
}
