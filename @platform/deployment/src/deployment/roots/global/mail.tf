locals {
  mail_zone = "643d8f0f0ea084d0c31c40098d7faec6"

  mail = {
    "aspmx" = { name = "symbol.industries", type = "MX", priority = 1, content = "aspmx.l.google.com" }
    "alt1"  = { name = "symbol.industries", type = "MX", priority = 5, content = "alt1.aspmx.l.google.com" }
    "alt2"  = { name = "symbol.industries", type = "MX", priority = 5, content = "alt2.aspmx.l.google.com" }
    "alt3"  = { name = "symbol.industries", type = "MX", priority = 10, content = "alt3.aspmx.l.google.com" }
    "alt4"  = { name = "symbol.industries", type = "MX", priority = 10, content = "alt4.aspmx.l.google.com" }
    "verification" = {
      name     = "symbol.industries"
      type     = "TXT"
      priority = null
      content  = "\"google-site-verification=tP2c7oe4L7CGCC3nk4JPwg-8rJ5BbADyCnql1xaYd1Q\""
    }
    "spf" = {
      name     = "symbol.industries"
      type     = "TXT"
      priority = null
      content  = "\"v=spf1 include:_spf.google.com ~all\""
    }
    "dmarc" = {
      name     = "_dmarc.symbol.industries"
      type     = "TXT"
      priority = null
      content  = "\"v=DMARC1; p=none; rua=mailto:florian@symbol.industries\""
    }
  }
}

resource "cloudflare_dns_record" "mail" {
  for_each = local.mail
  zone_id  = cloudflare_zone.domains["symbol.industries"].id
  name     = each.value.name
  type     = each.value.type
  priority = each.value.priority
  content  = each.value.content
  ttl      = 3600
}

import {
  for_each = {
    "aspmx"        = "a435263a0ef47eb37974bb02d32399eb"
    "alt1"         = "c2d79bf74bc9d6f0aea437692fe6425a"
    "alt2"         = "dbb3062de3cbd3f5342e07e4f5fbbc36"
    "alt3"         = "341f08e0eba8b4bab232546253062f88"
    "alt4"         = "8b13533176fd2e9591404a1c6aa623b7"
    "verification" = "117b1fb09b40b5c735f66dce1cad1998"
    "spf"          = "d07b71ca3709690ad20dead2ac4b9767"
  }
  to = cloudflare_dns_record.mail[each.key]
  id = "${local.mail_zone}/${each.value}"
}
