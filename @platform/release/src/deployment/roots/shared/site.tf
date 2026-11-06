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

# serve the setup guide at destack.me: Markdown to agents, the docs page to browsers
resource "cloudflare_dns_record" "setup" {
  for_each = toset(["destack.me", "www.destack.me"])
  zone_id  = cloudflare_zone.domains["destack.me"].id
  name     = each.key
  type     = "AAAA"
  content  = "100::"
  proxied  = true
  ttl      = 1
}

resource "cloudflare_workers_route" "setup" {
  pattern = "destack.me/*"
  script  = "destack-site"
  zone_id = cloudflare_zone.domains["destack.me"].id
}

# send www to the apex, and browsers and every other path to the setup page
resource "cloudflare_ruleset" "setup_redirect" {
  zone_id = cloudflare_zone.domains["destack.me"].id
  name    = "destack redirects"
  kind    = "zone"
  phase   = "http_request_dynamic_redirect"
  rules = [
    {
      action      = "redirect"
      description = "redirect www.destack.me to destack.me"
      expression  = "http.host eq \"www.destack.me\""
      enabled     = true
      action_parameters = {
        from_value = {
          status_code           = 301
          preserve_query_string = true
          target_url = {
            expression = "concat(\"https://destack.me\", http.request.uri.path)"
          }
        }
      }
    },
    {
      action      = "redirect"
      description = "redirect browsers and other paths to the setup page"
      expression  = "http.host eq \"destack.me\" and (http.request.uri.path ne \"/\" or any(http.request.headers[\"accept\"][*] contains \"text/html\"))"
      enabled     = true
      action_parameters = {
        from_value = {
          status_code = 302
          target_url = {
            value = "https://destack.sh/docs/setup/"
          }
        }
      }
    },
  ]
}

# answer agents at the apex with the setup guide's Markdown
resource "cloudflare_ruleset" "setup_rewrite" {
  zone_id = cloudflare_zone.domains["destack.me"].id
  name    = "destack setup"
  kind    = "zone"
  phase   = "http_request_transform"
  rules = [{
    action      = "rewrite"
    description = "serve the setup guide at destack.me"
    expression  = "http.host eq \"destack.me\" and http.request.uri.path eq \"/\""
    enabled     = true
    action_parameters = {
      uri = {
        path = { value = "/docs/setup.md" }
      }
    }
  }]
}

moved {
  from = cloudflare_dns_record.redirects["destack.me"]
  to   = cloudflare_dns_record.setup["destack.me"]
}

moved {
  from = cloudflare_dns_record.redirects["www.destack.me"]
  to   = cloudflare_dns_record.setup["www.destack.me"]
}

moved {
  from = cloudflare_ruleset.redirects["destack.me"]
  to   = cloudflare_ruleset.setup_redirect
}
