locals {
  domains = toset([
    "destack.app",
    "destack.blog",
    "destack.cloud",
    "destack.computer",
    "destack.design",
    "destack.dev",
    "destack.me",
    "destack.org",
    "destack.sh",
    "destack.site",
    "destack.software",
    "destack.space",
    "destack.studio",
    "destack.tech",
    "symbol.industries",
    "symbolx.com",
  ])

  redirects = {
    "destack.app" = {
      target = "destack.sh"
      hosts  = ["destack.app", "www.destack.app"]
    }
    "destack.blog" = {
      target = "destack.sh"
      hosts  = ["destack.blog", "www.destack.blog"]
    }
    "destack.cloud" = {
      target = "destack.app"
      hosts  = ["destack.cloud", "www.destack.cloud"]
    }
    "destack.computer" = {
      target = "destack.sh"
      hosts  = ["destack.computer", "www.destack.computer"]
    }
    "destack.design" = {
      target = "destack.sh"
      hosts  = ["destack.design", "www.destack.design"]
    }
    "destack.dev" = {
      target = "destack.sh"
      hosts  = ["destack.dev", "www.destack.dev"]
    }
    "destack.me" = {
      target = "destack.app"
      hosts  = ["destack.me", "www.destack.me"]
    }
    "destack.org" = {
      target = "destack.sh"
      hosts  = ["destack.org", "www.destack.org"]
    }
    "destack.sh" = {
      target = "destack.sh"
      hosts  = ["www.destack.sh"]
    }
    "destack.site" = {
      target = "destack.sh"
      hosts  = ["destack.site", "www.destack.site"]
    }
    "destack.software" = {
      target = "destack.sh"
      hosts  = ["destack.software", "www.destack.software"]
    }
    "destack.space" = {
      target = "destack.sh"
      hosts  = ["destack.space", "www.destack.space"]
    }
    "destack.studio" = {
      target = "destack.sh"
      hosts  = ["destack.studio", "www.destack.studio"]
    }
    "destack.tech" = {
      target = "destack.sh"
      hosts  = ["destack.tech", "www.destack.tech"]
    }
    "symbol.industries" = {
      target = "destack.sh"
      hosts  = ["symbol.industries", "www.symbol.industries"]
    }
  }
}

resource "cloudflare_zone" "domains" {
  for_each = local.domains
  account  = { id = local.account_id }
  name     = each.key
  type     = "full"

  lifecycle {
    prevent_destroy = true
  }
}

locals {
  redirect_hosts = merge([
    for name, zone in local.redirects : {
      for host in zone.hosts : host => merge(
        { zone_id = cloudflare_zone.domains[name].id },
        lookup(local.redirect_dns, host, { type = "AAAA", content = "100::" }),
      )
    }
  ]...)
}

resource "cloudflare_dns_record" "redirects" {
  for_each = local.redirect_hosts
  zone_id  = each.value.zone_id
  name     = each.key
  type     = each.value.type
  content  = each.value.content
  proxied  = true
  ttl      = 1
}

resource "cloudflare_ruleset" "redirects" {
  for_each = local.redirects
  zone_id  = cloudflare_zone.domains[each.key].id
  name     = "destack redirects"
  kind     = "zone"
  phase    = "http_request_dynamic_redirect"
  rules = [for host in each.value.hosts : {
    action      = "redirect"
    description = "redirect ${host} to ${each.value.target}"
    expression  = "http.host eq \"${host}\""
    enabled     = true
    action_parameters = {
      from_value = {
        status_code           = 301
        preserve_query_string = true
        target_url = {
          expression = "concat(\"https://${each.value.target}\", http.request.uri.path)"
        }
      }
    }
  }]
}

locals {
  redirect_dns = {
    "symbol.industries"     = { type = "A", content = "192.64.119.178" }
    "www.symbol.industries" = { type = "CNAME", content = "parkingpage.namecheap.com" }
  }
}
